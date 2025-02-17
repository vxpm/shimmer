//! Implementation of the R3000 CPU.

mod arith_logic;
mod coproc;
mod exception;
mod jump_branch;
mod load_store;

use crate::PSX;
use shimmer_core::{
    Cycles,
    cpu::{
        Reg, RegLoad,
        cop0::Exception,
        instr::{Instruction, SpecialCoOpcode},
    },
};
use shimmer_core::{
    cpu::instr::{CoOpcode, Opcode, SpecialOpcode},
    interrupts::Interrupt,
    kernel,
    mem::{Address, Region, io},
};
use std::hint::cold_path;
use tinylog::{debug, error, info, trace, warn};

// these are only the general exception vectors...
const EXCEPTION_VECTOR_KSEG0: Address = Address(0x8000_0080);
const EXCEPTION_VECTOR_KSEG1: Address = Address(0xBFC0_0180);

/// An interpreter of the R3000 CPU.
#[derive(Default)]
pub struct Interpreter {
    /// Address of the currently executing instruction.
    current_addr: Address,
    /// Value going to be loaded into a register after execution.
    pending_load: Option<RegLoad>,
}

const DEFAULT_DELAY: Cycles = 2;
const MEMORY_OP_DELAY: Cycles = 7;

impl Interpreter {
    #[cold]
    #[inline(never)]
    fn sideload(&mut self, psx: &mut PSX) {
        if let Some(exe) = &psx.memory.sideload {
            psx.cpu.instr_delay_slot = (Instruction::NOP, exe.header.initial_pc);
            psx.cpu.regs.write_pc(exe.header.initial_pc.value());
            psx.cpu.regs.write(Reg::GP, exe.header.initial_gp);

            let destination_ram =
                exe.header.destination.physical().unwrap().value() - Region::Ram.start().value();

            psx.memory.ram[destination_ram as usize..][..exe.header.length as usize]
                .copy_from_slice(&exe.program);

            if exe.header.initial_sp_base != 0 {
                let initial_sp = exe
                    .header
                    .initial_sp_base
                    .wrapping_add(exe.header.initial_sp_offset);
                psx.cpu.regs.write(Reg::SP, initial_sp);
            }

            info!(psx.loggers.cpu, "sideloaded!");
        }
    }

    fn trigger_exception_at(
        &mut self,
        psx: &mut PSX,
        address: Address,
        delay_slot: Address,
        exception: Exception,
    ) {
        let in_branch_delay = address.value().wrapping_add(4) != delay_slot.value();
        psx.cop0.regs.write(
            Reg::COP0_EPC,
            if in_branch_delay {
                address.value().wrapping_sub(4)
            } else {
                address.value()
            },
        );

        if exception != Exception::Interrupt {
            info!(
                psx.loggers.cpu,
                "triggered exception {:?} at {} (next would be: {})",
                exception,
                address,
                delay_slot;
                in_branch_delay = in_branch_delay,
            );
        } else {
            trace!(
                psx.loggers.cpu,
                "triggered exception {:?} at {} (next would be: {})",
                exception,
                address,
                delay_slot;
                in_branch_delay = in_branch_delay,
            );
        }

        // flush pipeline
        psx.cpu.instr_delay_slot = (Instruction::NOP, self.current_addr);

        // update sr
        psx.cop0.regs.system_status_mut().start_exception();

        // describe exception in cause
        psx.cop0
            .regs
            .cause_mut()
            .set_exception(exception)
            .set_branch_delay(in_branch_delay);

        // jump to exception handler indicated by BEV in system status
        // NOTE: this always jumps to the general exception handler... although others are very
        // unlikely to be used
        let exception_handler = if psx
            .cop0
            .regs
            .system_status()
            .boot_exception_vectors_in_kseg1()
        {
            EXCEPTION_VECTOR_KSEG1
        } else {
            EXCEPTION_VECTOR_KSEG0
        };

        psx.cpu.regs.write_pc(exception_handler.value());
    }

    /// Trigger an exception. This method should only be used inside instruction methods - if
    /// triggering an exception somewhere else, use [`trigger_exception_at`].
    fn trigger_exception(&mut self, psx: &mut PSX, exception: Exception) {
        self.trigger_exception_at(
            psx,
            self.current_addr,
            psx.cpu.instr_delay_slot.1,
            exception,
        );
    }

    /// Cancels a pending load to the given register, if it exists.
    fn cancel_load(&mut self, reg: Reg) {
        if self.pending_load.is_some_and(|load| load.reg == reg) {
            self.pending_load = None;
        }
    }

    fn check_interrupts(&mut self, psx: &mut PSX) -> bool {
        let masked_interrupt_status = psx.interrupts.status.mask(&psx.interrupts.mask);
        let requested_interrupt = masked_interrupt_status.requested();

        psx.cop0
            .regs
            .cause_mut()
            .set_system_interrupt_pending(requested_interrupt.is_some());

        if let Some(requested_interrupt) = requested_interrupt {
            let system_status = psx.cop0.regs.system_status();
            if !system_status.system_interrupts_enabled() {
                return false;
            }

            if requested_interrupt != Interrupt::VBlank {
                info!(
                    psx.loggers.cpu,
                    "triggered interrupt {:?} at {}",
                    requested_interrupt, psx.cpu.instr_delay_slot.1;
                );
            }

            self.trigger_exception(psx, Exception::Interrupt);

            true
        } else {
            false
        }
    }

    fn exec(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        if let Some(op) = instr.op() {
            match op {
                Opcode::LUI => self.lui(psx, instr),
                Opcode::ORI => self.ori(psx, instr),
                Opcode::SW => self.sw(psx, instr),
                Opcode::ADDIU => self.addiu(psx, instr),
                Opcode::JMP => self.jmp(psx, instr),
                Opcode::BNE => self.bne(psx, instr),
                Opcode::ADDI => self.addi(psx, instr),
                Opcode::LW => self.lw(psx, instr),
                Opcode::SH => self.sh(psx, instr),
                Opcode::JAL => self.jal(psx, instr),
                Opcode::ANDI => self.andi(psx, instr),
                Opcode::SB => self.sb(psx, instr),
                Opcode::LB => self.lb(psx, instr),
                Opcode::BEQ => self.beq(psx, instr),
                Opcode::BGTZ => self.bgtz(psx, instr),
                Opcode::BLEZ => self.blez(psx, instr),
                Opcode::LBU => self.lbu(psx, instr),
                Opcode::BZ => self.bz(psx, instr),
                Opcode::SLTI => self.slti(psx, instr),
                Opcode::SLTIU => self.sltiu(psx, instr),
                Opcode::LHU => self.lhu(psx, instr),
                Opcode::LH => self.lh(psx, instr),
                Opcode::LWL => self.lwl(psx, instr),
                Opcode::LWR => self.lwr(psx, instr),
                Opcode::SWL => self.swl(psx, instr),
                Opcode::SWR => self.swr(psx, instr),
                Opcode::XORI => self.xori(psx, instr),
                Opcode::COP2 => {
                    warn!(psx.loggers.cpu, "ignoring GTE instruction");
                    DEFAULT_DELAY
                }
                Opcode::COP0 | Opcode::COP1 | Opcode::COP3 => {
                    if let Some(op) = instr.cop_op() {
                        match op {
                            CoOpcode::MFC => self.mfc(psx, instr),
                            CoOpcode::CFC => todo!(),
                            CoOpcode::MTC => self.mtc(psx, instr),
                            CoOpcode::CTC => todo!("{:?}", instr.cop()),
                            CoOpcode::BRANCH => todo!(),
                            CoOpcode::SPECIAL => {
                                if let Some(op) = instr.cop_special_op() {
                                    match op {
                                        SpecialCoOpcode::RFE => self.rfe(psx, instr),
                                    }
                                } else {
                                    DEFAULT_DELAY
                                }
                            }
                        }
                    } else {
                        DEFAULT_DELAY
                    }
                }
                Opcode::SWC0 | Opcode::SWC1 | Opcode::SWC2 | Opcode::SWC3 => self.swc(psx, instr),
                Opcode::SPECIAL => {
                    if let Some(op) = instr.special_op() {
                        match op {
                            SpecialOpcode::SLL => self.sll(psx, instr),
                            SpecialOpcode::OR => self.or(psx, instr),
                            SpecialOpcode::SLTU => self.sltu(psx, instr),
                            SpecialOpcode::ADDU => self.addu(psx, instr),
                            SpecialOpcode::JR => self.jr(psx, instr),
                            SpecialOpcode::JALR => self.jalr(psx, instr),
                            SpecialOpcode::SRL => self.srl(psx, instr),
                            SpecialOpcode::AND => self.and(psx, instr),
                            SpecialOpcode::ADD => self.add(psx, instr),
                            SpecialOpcode::SUBU => self.subu(psx, instr),
                            SpecialOpcode::SRA => self.sra(psx, instr),
                            SpecialOpcode::DIV => self.div(psx, instr),
                            SpecialOpcode::MFLO => self.mflo(psx, instr),
                            SpecialOpcode::SYSCALL => self.syscall(psx, instr),
                            SpecialOpcode::MFHI => self.mfhi(psx, instr),
                            SpecialOpcode::MTLO => self.mtlo(psx, instr),
                            SpecialOpcode::MTHI => self.mthi(psx, instr),
                            SpecialOpcode::SLT => self.slt(psx, instr),
                            SpecialOpcode::DIVU => self.divu(psx, instr),
                            SpecialOpcode::SLLV => self.sllv(psx, instr),
                            SpecialOpcode::NOR => self.nor(psx, instr),
                            SpecialOpcode::SRAV => self.srav(psx, instr),
                            SpecialOpcode::SRLV => self.srlv(psx, instr),
                            SpecialOpcode::MULTU => self.multu(psx, instr),
                            SpecialOpcode::XOR => self.xor(psx, instr),
                            SpecialOpcode::MULT => self.mult(psx, instr),
                            SpecialOpcode::SUB => self.sub(psx, instr),
                            SpecialOpcode::BREAK => self.breakpoint(psx, instr),
                        }
                    } else {
                        error!(psx.loggers.cpu, "illegal special op");
                        DEFAULT_DELAY
                    }
                }
                _ => {
                    error!(psx.loggers.cpu, "can't execute op {op:?}");
                    DEFAULT_DELAY
                }
            }
        } else {
            error!(psx.loggers.cpu, "illegal op");
            DEFAULT_DELAY
        }
    }

    fn log_kernel_calls(&mut self, psx: &mut PSX) {
        let func = match self.current_addr.value() {
            0xA0 => {
                cold_path();
                let code = psx.cpu.regs.read(Reg::R9) as u8;
                kernel::Function::a0(code)
            }
            0xB0 => {
                cold_path();
                let code = psx.cpu.regs.read(Reg::R9) as u8;
                kernel::Function::b0(code)
            }
            0xC0 => {
                cold_path();
                let code = psx.cpu.regs.read(Reg::R9) as u8;
                kernel::Function::c0(code)
            }
            _ => return,
        };

        if let Some(func) = func {
            if func == kernel::Function::PutChar {
                let char = psx.cpu.regs.read(Reg::A0);
                if let Ok(char) = char::try_from(char) {
                    print!("{char}");
                    if char == '\r' {
                        psx.memory.kernel_stdout.push('\n');
                    } else {
                        psx.memory.kernel_stdout.push(char);
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
                1 => vec![psx.cpu.regs.read(Reg::A0)],
                2 => vec![psx.cpu.regs.read(Reg::A0), psx.cpu.regs.read(Reg::A1)],
                3 => vec![
                    psx.cpu.regs.read(Reg::A0),
                    psx.cpu.regs.read(Reg::A1),
                    psx.cpu.regs.read(Reg::A2),
                ],
                _ => vec![
                    psx.cpu.regs.read(Reg::A0),
                    psx.cpu.regs.read(Reg::A1),
                    psx.cpu.regs.read(Reg::A2),
                    psx.cpu.regs.read(Reg::A3),
                ],
            };

            let args = args
                .into_iter()
                .map(|x| format!("0x{x:08X}"))
                .collect::<Vec<_>>()
                .join(", ");

            debug!(
                psx.loggers.kernel,
                "executed kernel function {func:?}({args})"
            );
        } else {
            let code = psx.cpu.regs.read(Reg::R9) as u8;
            warn!(
                psx.loggers.kernel,
                "executed unknown kernel function 0x{:02X} at {}", code, self.current_addr
            );
        }
    }

    /// Executes the next instruction and returns how many cycles it takes to complete.
    pub fn exec_next(&mut self, psx: &mut PSX) -> u64 {
        if psx.cpu.instr_delay_slot.1.value() == 0x8003_0000 {
            cold_path();
            self.sideload(psx);
        }

        let pc = Address(psx.cpu.regs.read_pc());
        let Ok(fetched) = psx.read::<_, true>(pc) else {
            if let Some(load) = psx.cpu.load_delay_slot.take() {
                psx.cpu.regs.write(load.reg, load.value);
            }
            if let Some(load) = psx.cop0.load_delay_slot.take() {
                psx.cop0.regs.write(load.reg, load.value);
            }

            self.trigger_exception_at(
                psx,
                psx.cpu.instr_delay_slot.1,
                psx.cpu.regs.read_pc().into(),
                Exception::AddressErrorLoad,
            );
            return DEFAULT_DELAY;
        };

        let (current_instr, current_addr) = std::mem::replace(
            &mut psx.cpu.instr_delay_slot,
            (Instruction::from_bits(fetched), pc),
        );

        self.current_addr = current_addr;
        psx.cpu
            .regs
            .write_pc(psx.cpu.regs.read_pc().wrapping_add(4));

        self.log_kernel_calls(psx);

        self.pending_load = psx.cpu.load_delay_slot.take();
        let pending_load_cop0 = psx.cop0.load_delay_slot.take();

        let cycles = if !self.check_interrupts(psx) {
            self.exec(psx, current_instr)
        } else {
            DEFAULT_DELAY
        };

        if let Some(load) = self.pending_load {
            psx.cpu.regs.write(load.reg, load.value);
        }

        if let Some(load) = pending_load_cop0 {
            psx.cop0.regs.write(load.reg, load.value);
        }

        if let Some(physical) = pc.physical()
            && (physical.region() == Some(Region::ScratchPad)
                || physical == io::Reg::InterruptStatus.address()
                || physical == io::Reg::InterruptMask.address())
        {
            self.trigger_exception_at(
                psx,
                psx.cpu.instr_delay_slot.1,
                psx.cpu.regs.read_pc().into(),
                Exception::BusErrorInstruction,
            );
            return DEFAULT_DELAY;
        }

        cycles
    }
}
