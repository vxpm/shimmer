//! An interpreter for the R3000 CPU.

mod arith_logic;
mod coproc;
mod exception;
mod jump_branch;
mod load_store;

use super::{
    Reg, RegLoad,
    cop0::Exception,
    instr::{Instruction, SpecialCoOpcode},
};
use crate::{
    PSX,
    cpu::{
        EXCEPTION_VECTOR_KSEG0, EXCEPTION_VECTOR_KSEG1,
        instr::{CoOpcode, Opcode, SpecialOpcode},
    },
    interrupts::Interrupt,
    kernel,
    mem::{Address, Region, io},
    util::cold_path,
};
use tinylog::{debug, error, info, trace, warn};

/// An interpreter of the R3000 CPU. This struct does not have any persistent state and is mostly
/// just convenience for the implementation. It is intended to be created whenever you want to
/// execute an instruction.
pub struct Interpreter<'ctx> {
    psx: &'ctx mut PSX,
    /// Address of the currently executing instruction.
    current_addr: Address,
    /// Value going to be loaded into a register after execution.
    pending_load: Option<RegLoad>,
}

const DEFAULT_CYCLE_COUNT: u64 = 2;

impl<'ctx> Interpreter<'ctx> {
    #[inline(always)]
    pub fn new(psx: &'ctx mut PSX) -> Self {
        Self {
            psx,
            current_addr: Default::default(),
            pending_load: None,
        }
    }

    #[cold]
    #[inline(never)]
    fn sideload(&mut self) {
        if let Some(exe) = &self.psx.memory.sideload {
            self.psx.cpu.instr_delay_slot = (Instruction::NOP, exe.header.initial_pc);
            self.psx.cpu.regs.pc = exe.header.initial_pc.value();
            self.psx.cpu.regs.write(Reg::GP, exe.header.initial_gp);

            let destination_ram =
                exe.header.destination.physical().unwrap().value() - Region::Ram.start().value();

            self.psx.memory.ram[destination_ram as usize..][..exe.header.length as usize]
                .copy_from_slice(&exe.program);

            if exe.header.initial_sp_base != 0 {
                let initial_sp = exe
                    .header
                    .initial_sp_base
                    .wrapping_add(exe.header.initial_sp_offset);
                self.psx.cpu.regs.write(Reg::SP, initial_sp);
            }

            info!(self.psx.loggers.cpu, "sideloaded!");
        }

        // write args
        let args = [c"auto", c"console", c"release"];
        self.psx
            .write::<_, true>(Address(0x1F80_0000), args.len() as u32)
            .unwrap();

        let mut offset = 0;
        for (index, arg) in args.iter().enumerate() {
            self.psx
                .write::<_, true>(
                    Address(0x1F80_0004 + index as u32 * 4),
                    0x1F80_0044 + offset,
                )
                .unwrap();

            for &byte in arg.to_bytes_with_nul() {
                self.psx
                    .write::<_, true>(Address(0x1F80_0044 + offset), byte)
                    .unwrap();
                offset += 1;
            }
        }
    }

    fn trigger_exception_at(
        &mut self,
        address: Address,
        delay_slot: Address,
        exception: Exception,
    ) {
        let in_branch_delay = address.value().wrapping_add(4) != delay_slot.value();
        self.psx.cop0.regs.write(
            Reg::COP0_EPC,
            if in_branch_delay {
                address.value().wrapping_sub(4)
            } else {
                address.value()
            },
        );

        if exception != Exception::Interrupt {
            info!(
                self.psx.loggers.cpu,
                "triggered exception {:?} at {} (next would be: {})",
                exception,
                address,
                delay_slot;
                in_branch_delay = in_branch_delay,
            );
        } else {
            trace!(
                self.psx.loggers.cpu,
                "triggered exception {:?} at {} (next would be: {})",
                exception,
                address,
                delay_slot;
                in_branch_delay = in_branch_delay,
            );
        }

        // flush pipeline
        self.psx.cpu.instr_delay_slot = (Instruction::NOP, self.current_addr);

        // update sr
        self.psx.cop0.regs.system_status_mut().start_exception();

        // describe exception in cause
        self.psx
            .cop0
            .regs
            .cause_mut()
            .set_exception(exception)
            .set_branch_delay(in_branch_delay);

        // jump to exception handler indicated by BEV in system status
        // NOTE: this always jumps to the general exception handler... although others are very
        // unlikely to be used
        let exception_handler = if self
            .psx
            .cop0
            .regs
            .system_status()
            .boot_exception_vectors_in_kseg1()
        {
            EXCEPTION_VECTOR_KSEG1
        } else {
            EXCEPTION_VECTOR_KSEG0
        };

        self.psx.cpu.regs.pc = exception_handler.value();
    }

    /// Trigger an exception. This method should only be used inside instruction methods - if
    /// triggering an exception somewhere else, use [`trigger_exception_at`].
    fn trigger_exception(&mut self, exception: Exception) {
        self.trigger_exception_at(
            self.current_addr,
            self.psx.cpu.instr_delay_slot.1,
            exception,
        );
    }

    /// Cancels a pending load to the given register, if it exists.
    fn cancel_load(&mut self, reg: Reg) {
        if self.pending_load.is_some_and(|load| load.reg == reg) {
            self.pending_load = None;
        }
    }

    fn check_interrupts(&mut self) -> bool {
        let masked_interrupt_status = self.psx.interrupts.status.mask(&self.psx.interrupts.mask);
        let requested_interrupt = masked_interrupt_status.requested();

        self.psx
            .cop0
            .regs
            .cause_mut()
            .set_system_interrupt_pending(requested_interrupt.is_some());

        if let Some(requested_interrupt) = requested_interrupt {
            // must have SR.BIT10 == 1
            let system_status = self.psx.cop0.regs.system_status();
            if !system_status.system_interrupts_enabled() {
                return false;
            }

            if requested_interrupt != Interrupt::VBlank {
                info!(
                    self.psx.loggers.cpu,
                    "triggered interrupt {:?} at {}",
                    requested_interrupt, self.psx.cpu.instr_delay_slot().1;
                );
            }

            self.trigger_exception(Exception::Interrupt);

            true
        } else {
            false
        }
    }

    fn exec(&mut self, instr: Instruction) -> u64 {
        if let Some(op) = instr.op() {
            match op {
                Opcode::LUI => self.lui(instr),
                Opcode::ORI => self.ori(instr),
                Opcode::SW => self.sw(instr),
                Opcode::ADDIU => self.addiu(instr),
                Opcode::JMP => self.jmp(instr),
                Opcode::BNE => self.bne(instr),
                Opcode::ADDI => self.addi(instr),
                Opcode::LW => self.lw(instr),
                Opcode::SH => self.sh(instr),
                Opcode::JAL => self.jal(instr),
                Opcode::ANDI => self.andi(instr),
                Opcode::SB => self.sb(instr),
                Opcode::LB => self.lb(instr),
                Opcode::BEQ => self.beq(instr),
                Opcode::BGTZ => self.bgtz(instr),
                Opcode::BLEZ => self.blez(instr),
                Opcode::LBU => self.lbu(instr),
                Opcode::BZ => self.bz(instr),
                Opcode::SLTI => self.slti(instr),
                Opcode::SLTIU => self.sltiu(instr),
                Opcode::LHU => self.lhu(instr),
                Opcode::LH => self.lh(instr),
                Opcode::LWL => self.lwl(instr),
                Opcode::LWR => self.lwr(instr),
                Opcode::SWL => self.swl(instr),
                Opcode::SWR => self.swr(instr),
                Opcode::XORI => self.xori(instr),
                Opcode::COP2 => {
                    warn!(self.psx.loggers.cpu, "ignoring GTE instruction");
                    DEFAULT_CYCLE_COUNT
                }
                Opcode::COP0 | Opcode::COP1 | Opcode::COP3 => {
                    if let Some(op) = instr.cop_op() {
                        match op {
                            CoOpcode::MFC => self.mfc(instr),
                            CoOpcode::CFC => todo!(),
                            CoOpcode::MTC => self.mtc(instr),
                            CoOpcode::CTC => todo!("{:?}", instr.cop()),
                            CoOpcode::BRANCH => todo!(),
                            CoOpcode::SPECIAL => {
                                if let Some(op) = instr.cop_special_op() {
                                    match op {
                                        SpecialCoOpcode::RFE => self.rfe(instr),
                                    }
                                } else {
                                    DEFAULT_CYCLE_COUNT
                                }
                            }
                        }
                    } else {
                        DEFAULT_CYCLE_COUNT
                    }
                }
                Opcode::SWC0 | Opcode::SWC1 | Opcode::SWC2 | Opcode::SWC3 => self.swc(instr),
                Opcode::SPECIAL => {
                    if let Some(op) = instr.special_op() {
                        match op {
                            SpecialOpcode::SLL => self.sll(instr),
                            SpecialOpcode::OR => self.or(instr),
                            SpecialOpcode::SLTU => self.sltu(instr),
                            SpecialOpcode::ADDU => self.addu(instr),
                            SpecialOpcode::JR => self.jr(instr),
                            SpecialOpcode::JALR => self.jalr(instr),
                            SpecialOpcode::SRL => self.srl(instr),
                            SpecialOpcode::AND => self.and(instr),
                            SpecialOpcode::ADD => self.add(instr),
                            SpecialOpcode::SUBU => self.subu(instr),
                            SpecialOpcode::SRA => self.sra(instr),
                            SpecialOpcode::DIV => self.div(instr),
                            SpecialOpcode::MFLO => self.mflo(instr),
                            SpecialOpcode::SYSCALL => self.syscall(instr),
                            SpecialOpcode::MFHI => self.mfhi(instr),
                            SpecialOpcode::MTLO => self.mtlo(instr),
                            SpecialOpcode::MTHI => self.mthi(instr),
                            SpecialOpcode::SLT => self.slt(instr),
                            SpecialOpcode::DIVU => self.divu(instr),
                            SpecialOpcode::SLLV => self.sllv(instr),
                            SpecialOpcode::NOR => self.nor(instr),
                            SpecialOpcode::SRAV => self.srav(instr),
                            SpecialOpcode::SRLV => self.srlv(instr),
                            SpecialOpcode::MULTU => self.multu(instr),
                            SpecialOpcode::XOR => self.xor(instr),
                            SpecialOpcode::MULT => self.mult(instr),
                            SpecialOpcode::SUB => self.sub(instr),
                            SpecialOpcode::BREAK => self.breakpoint(instr),
                        }
                    } else {
                        error!(self.psx.loggers.cpu, "illegal special op");
                        DEFAULT_CYCLE_COUNT
                    }
                }
                _ => {
                    error!(self.psx.loggers.cpu, "can't execute op {op:?}");
                    DEFAULT_CYCLE_COUNT
                }
            }
        } else {
            error!(self.psx.loggers.cpu, "illegal op");
            DEFAULT_CYCLE_COUNT
        }
    }

    fn log_kernel_calls(&mut self) {
        let func = match self.current_addr.value() {
            0xA0 => {
                cold_path();
                let code = self.psx.cpu.regs.read(Reg::R9) as u8;
                kernel::Function::a0(code)
            }
            0xB0 => {
                cold_path();
                let code = self.psx.cpu.regs.read(Reg::R9) as u8;
                kernel::Function::b0(code)
            }
            0xC0 => {
                cold_path();
                let code = self.psx.cpu.regs.read(Reg::R9) as u8;
                kernel::Function::c0(code)
            }
            _ => return,
        };

        if let Some(func) = func {
            if func == kernel::Function::PutChar {
                let char = self.psx.cpu.regs().read(Reg::A0);
                if let Ok(char) = char::try_from(char) {
                    print!("{char}");
                    if char == '\r' {
                        self.psx.memory.kernel_stdout.push('\n');
                    } else {
                        self.psx.memory.kernel_stdout.push(char);
                    }
                }

                return;
            }

            let ignore = [
                kernel::Function::Rand,
                kernel::Function::ReturnFromException,
                kernel::Function::TestEvent,
            ];

            if ignore.contains(&func) {
                return;
            }

            let args = match func.args() {
                0 => vec![],
                1 => vec![self.psx.cpu.regs.read(Reg::A0)],
                2 => vec![
                    self.psx.cpu.regs.read(Reg::A0),
                    self.psx.cpu.regs.read(Reg::A1),
                ],
                3 => vec![
                    self.psx.cpu.regs.read(Reg::A0),
                    self.psx.cpu.regs.read(Reg::A1),
                    self.psx.cpu.regs.read(Reg::A2),
                ],
                _ => vec![
                    self.psx.cpu.regs.read(Reg::A0),
                    self.psx.cpu.regs.read(Reg::A1),
                    self.psx.cpu.regs.read(Reg::A2),
                    self.psx.cpu.regs.read(Reg::A3),
                ],
            };

            let args = args
                .into_iter()
                .map(|x| format!("0x{x:08X}"))
                .collect::<Vec<_>>()
                .join(", ");

            debug!(
                self.psx.loggers.kernel,
                "executed kernel function {func:?}({args})"
            );
        } else {
            let code = self.psx.cpu.regs.read(Reg::R9) as u8;
            warn!(
                self.psx.loggers.kernel,
                "executed unknown kernel function 0x{:02X} at {}", code, self.current_addr
            );
        }
    }

    /// Executes the next instruction and returns how many cycles it takes to complete.
    pub fn exec_next(&mut self) -> u64 {
        if self.psx.cpu.instr_delay_slot.1.value() == 0x8003_0000 {
            cold_path();
            self.sideload();
        }

        let pc = Address(self.psx.cpu.regs.pc);
        let Ok(fetched) = self.psx.read::<_, true>(pc) else {
            if let Some(load) = self.psx.cpu.load_delay_slot.take() {
                self.psx.cpu.regs.write(load.reg, load.value);
            }
            if let Some(load) = self.psx.cop0.load_delay_slot.take() {
                self.psx.cop0.regs.write(load.reg, load.value);
            }

            self.trigger_exception_at(
                self.psx.cpu.instr_delay_slot.1,
                Address(self.psx.cpu.regs.pc),
                Exception::AddressErrorLoad,
            );
            return DEFAULT_CYCLE_COUNT;
        };

        let (current_instr, current_addr) = std::mem::replace(
            &mut self.psx.cpu.instr_delay_slot,
            (Instruction::from_bits(fetched), pc),
        );

        self.current_addr = current_addr;
        self.psx.cpu.regs.pc = self.psx.cpu.regs.pc.wrapping_add(4);

        self.log_kernel_calls();

        self.pending_load = self.psx.cpu.load_delay_slot.take();
        let pending_load_cop0 = self.psx.cop0.load_delay_slot.take();

        let cycles = if !self.check_interrupts() {
            self.exec(current_instr)
        } else {
            DEFAULT_CYCLE_COUNT
        };

        if let Some(load) = self.pending_load {
            self.psx.cpu.regs.write(load.reg, load.value);
        }

        if let Some(load) = pending_load_cop0 {
            self.psx.cop0.regs.write(load.reg, load.value);
        }

        if let Some(physical) = pc.physical()
            && (physical.region() == Some(Region::ScratchPad)
                || physical == io::Reg::InterruptStatus.address()
                || physical == io::Reg::InterruptMask.address())
        {
            self.trigger_exception_at(
                self.psx.cpu.instr_delay_slot.1,
                Address(self.psx.cpu.regs.pc),
                Exception::BusErrorInstruction,
            );
            return DEFAULT_CYCLE_COUNT;
        }

        cycles
    }
}
