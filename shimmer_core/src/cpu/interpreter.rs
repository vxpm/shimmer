mod arith_logic;
mod coproc;
mod exception;
mod jump_branch;
mod load_store;

use super::{
    Reg,
    cop0::Exception,
    instr::{Instruction, SpecialCoOpcode},
};
use crate::{
    cpu::instr::{CoOpcode, Opcode, SpecialOpcode},
    kernel,
    mem::{self, Address, Region},
};
use tinylog::{debug, error, info, warn};

pub struct Interpreter<'ctx> {
    bus: mem::Bus<'ctx>,
    /// Address of the currently executing instruction.
    current_addr: Address,
}

// these are only the general exception vectors...
pub const EXCEPTION_VECTOR_KSEG0: Address = Address(0x8000_0080);
pub const EXCEPTION_VECTOR_KSEG1: Address = Address(0xBFC0_0180);

impl<'ctx> Interpreter<'ctx> {
    pub fn new(bus: mem::Bus<'ctx>) -> Self {
        Self {
            bus,
            current_addr: Default::default(),
        }
    }

    fn sideload(&mut self) {
        if let Some(exe) = &self.bus.memory.sideload {
            self.bus.cpu.regs.pc = exe.header.initial_pc.value();
            self.bus.cpu.regs.write(Reg::GP, exe.header.initial_gp);

            let destination_ram =
                exe.header.destination.physical().unwrap().value() - Region::Ram.start().value();

            self.bus.memory.ram[destination_ram as usize..][..exe.header.length as usize]
                .copy_from_slice(&exe.program);

            if exe.header.initial_sp_base != 0 {
                let initial_sp = exe
                    .header
                    .initial_sp_base
                    .wrapping_add(exe.header.initial_sp_offset);
                self.bus.cpu.regs.write(Reg::SP, initial_sp);
            }
        }
    }

    /// Trigger an exception.
    fn trigger_exception(&mut self, exception: Exception) {
        // store return address in EPC and clear next instruction (exceptions
        // have no delay slot)
        //
        // if we are in a delay slot, we must save the address of the previous instruction as the
        // return address. this is because, if we were to take the branch, returning to the delay
        // slot would not take the branch anymore! so we must execute the branch again in order to
        // avoid this.

        // HACK: ideally should be set by instructions (in_branch_delay)
        let in_branch_delay = (self.bus.cpu.regs.pc - self.current_addr.value()) != 4;

        self.bus.cop0.regs.write(
            Reg::COP0_EPC,
            if in_branch_delay {
                self.current_addr.value().saturating_sub(4)
            } else {
                self.current_addr.value()
            },
        );

        // flush pipeline
        self.bus.cpu.to_exec = (Instruction::NOP, self.current_addr);

        // update sr
        self.bus.cop0.regs.system_status_mut().start_exception();

        // describe exception in cause
        self.bus
            .cop0
            .regs
            .cause_mut()
            .set_exception(exception)
            .set_branch_delay(in_branch_delay);

        // jump to exception handler indicated by BEV in system status
        // TODO: this always jumps to the general exception handler... although others are very
        // unlikely to be used
        let exception_handler = if self
            .bus
            .cop0
            .regs
            .system_status()
            .boot_exception_vectors_in_kseg1()
        {
            EXCEPTION_VECTOR_KSEG1
        } else {
            EXCEPTION_VECTOR_KSEG0
        };

        info!(
            self.bus.loggers.cpu,
            "triggered exception {:?} at {}. in_branch_delay={:?}, sr={:?}, exception_handler={:?}",
            exception,
            self.current_addr,
            in_branch_delay,
            self.bus.cop0.regs.system_status().clone(),
            exception_handler
        );

        self.bus.cpu.regs.pc = exception_handler.value();
    }

    fn check_interrupts(&mut self) {
        // (I_STAT & I_MASK)
        let masked_interrupt_status = self
            .bus
            .cop0
            .interrupt_status
            .mask(&self.bus.cop0.interrupt_mask);

        // get interrupt if != 0
        let requested_interrupt = masked_interrupt_status.requested();

        // update CAUSE
        self.bus
            .cop0
            .regs
            .cause_mut()
            .set_interrupt_pending(requested_interrupt.is_some());

        if let Some(requested_interrupt) = requested_interrupt {
            // must have SR.BIT10 == 1
            let system_status = self.bus.cop0.regs.system_status();
            if !system_status.interrupts_enabled() {
                return;
            }

            info!(
                self.bus.loggers.cpu,
                "triggered interrupt {:?} @ {}",
                requested_interrupt, self.current_addr;
                interrupt = requested_interrupt,
                address = self.current_addr,
            );

            self.trigger_exception(Exception::Interrupt);
        }
    }

    fn exec(&mut self, instr: Instruction) {
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
                Opcode::COP0 | Opcode::COP2 => {
                    if let Some(op) = instr.cop_op() {
                        match op {
                            CoOpcode::MFC => self.mfc(instr),
                            CoOpcode::CFC => todo!(),
                            CoOpcode::MTC => self.mtc(instr),
                            CoOpcode::CTC => todo!(),
                            CoOpcode::BRANCH => todo!(),
                            CoOpcode::SPECIAL => {
                                if let Some(op) = instr.cop_special_op() {
                                    match op {
                                        SpecialCoOpcode::RFE => self.rfe(instr),
                                    }
                                }
                            }
                        }
                    }
                }
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
                            _ => error!(self.bus.loggers.cpu, "can't execute special op {op:?}"),
                        }
                    } else {
                        error!(self.bus.loggers.cpu, "illegal special op");
                    }
                }
                _ => {
                    error!(self.bus.loggers.cpu, "can't execute op {op:?}")
                }
            }
        } else {
            error!(self.bus.loggers.cpu, "illegal op");
        }
    }

    fn log_kernel_calls(&self) {
        let code = self.bus.cpu.regs.read(Reg::R9) as u8;
        let func = match self.current_addr.value() {
            0xA0 => kernel::Function::a0(code),
            0xB0 => kernel::Function::b0(code),
            0xC0 => kernel::Function::c0(code),
            _ => return,
        };

        if let Some(func) = func {
            let args = match func.args() {
                0 => vec![],
                1 => vec![self.bus.cpu.regs.read(Reg::A0)],
                2 => vec![
                    self.bus.cpu.regs.read(Reg::A0),
                    self.bus.cpu.regs.read(Reg::A1),
                ],
                3 => vec![
                    self.bus.cpu.regs.read(Reg::A0),
                    self.bus.cpu.regs.read(Reg::A1),
                    self.bus.cpu.regs.read(Reg::A2),
                ],
                4 => vec![
                    self.bus.cpu.regs.read(Reg::A0),
                    self.bus.cpu.regs.read(Reg::A1),
                    self.bus.cpu.regs.read(Reg::A2),
                    self.bus.cpu.regs.read(Reg::A3),
                ],
                _ => vec![
                    self.bus.cpu.regs.read(Reg::A0),
                    self.bus.cpu.regs.read(Reg::A1),
                    self.bus.cpu.regs.read(Reg::A2),
                    self.bus.cpu.regs.read(Reg::A3),
                ],
            };

            let args = args
                .into_iter()
                .map(|x| format!("0x{x:08X}"))
                .collect::<Vec<_>>()
                .join(", ");

            debug!(
                self.bus.loggers.kernel,
                "executed kernel function {func:?}({args})"
            );
            if func != kernel::Function::PutChar {}
        } else {
            warn!(
                self.bus.loggers.kernel,
                "executed unknown kernel function 0x{:02X} at {}", code, self.current_addr
            );
        }
    }

    pub fn cycle(&mut self) {
        let pc_addr = Address(self.bus.cpu.regs.pc);
        let fetched = self.bus.read::<_, true>(pc_addr).expect("pc is aligned");

        let (instr, instr_addr) = std::mem::replace(
            &mut self.bus.cpu.to_exec,
            (Instruction::from_bits(fetched), pc_addr),
        );
        self.current_addr = instr_addr;

        self.log_kernel_calls();

        if instr_addr.value() == 0x8003_0000 {
            self.sideload();
        }

        let to_load = self.bus.cpu.to_load.take();

        self.exec(instr);

        if let Some((reg, value)) = to_load {
            self.bus.cpu.regs.write(reg, value);
        }

        self.bus.cpu.regs.pc = self.bus.cpu.regs.pc.wrapping_add(4);
        self.check_interrupts();
    }

    #[inline(always)]
    pub fn cycle_n(&mut self, count: u64) {
        for _ in 0..count {
            self.cycle();
        }
    }
}

#[cfg(test)]
pub mod test {
    use crate::cpu::{COP, Reg};
    use proptest::prelude::*;

    pub fn any_cop() -> impl Strategy<Value = COP> {
        // TODO: COP2
        any::<bool>().prop_map(|_b| COP::COP0)
    }

    pub fn cpu_regs() -> impl Strategy<Value = crate::cpu::Registers> {
        (any::<[u32; 32]>(), any::<u32>(), any::<u32>()).prop_map(|(mut regs, hi, lo)| {
            regs[0] = 0;
            crate::cpu::Registers {
                gp: regs,
                hi,
                lo,
                pc: 0,
            }
        })
    }

    pub fn cop0_regs() -> impl Strategy<Value = crate::cpu::cop0::Registers> {
        (any::<[u32; 32]>()).prop_map(crate::cpu::cop0::Registers)
    }

    pub type TestState = (crate::cpu::Registers, crate::cop0::Registers);

    pub fn state() -> impl Strategy<Value = TestState> {
        (cpu_regs(), cop0_regs())
    }

    pub fn any_reg() -> impl Strategy<Value = Reg> {
        any::<Reg>()
    }

    pub fn any_writable_reg() -> impl Strategy<Value = Reg> {
        any::<Reg>().prop_filter("must be a writable register", |r| *r != Reg::ZERO)
    }

    macro_rules! test_interpreter {
        ($interpreter:ident($regs:ident) => $($code:tt)*) => {
            let code = $crate::mips! { $($code)* };

            let (cpu_regs, cop0_regs) = $regs;

            let mut cpu = $crate::cpu::State::default();
            cpu.regs = cpu_regs;

            let mut cop0 = $crate::cpu::cop0::State::default();
            cop0.regs = cop0_regs;

            let mut gpu = $crate::gpu::State::default();

            let mut memory = $crate::mem::Memory::with_bios(vec![]).unwrap();
            let mut bus = $crate::mem::Bus {
                memory: &mut memory,
                cpu: &mut cpu,
                cop0: &mut cop0,
                gpu: &mut gpu,
                loggers: &mut $crate::Loggers::new(::tinylog::Logger::dummy()),
            };

            for (i, byte) in code.into_iter().flat_map(|i| i.into_bits().to_le_bytes()).enumerate() {
                bus.write::<_, false>($crate::mem::Address(bus.cpu.regs.pc.wrapping_add(i as u32)), byte).unwrap();
            }

            let mut $interpreter = $crate::cpu::Interpreter::new(bus);
        };
    }

    pub(crate) use test_interpreter;
}
