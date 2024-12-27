mod arith_logic;
mod coproc;
mod jump_branch;
mod load_store;

use super::{
    Reg,
    cop0::Exception,
    instr::{Instruction, SpecialCoOpcode},
};
use crate::{
    cpu::instr::{CoOpcode, Opcode, SpecialOpcode},
    mem::{self, Address},
};

pub struct Interpreter<'ctx> {
    bus: mem::Bus<'ctx>,
    /// Address of the currently executing instruction.
    current_addr: Address,
}

// these are only the general exception vectors...
pub const EXCEPTION_VECTOR_KSEG0: u32 = 0x8000_0080;
pub const EXCEPTION_VECTOR_KSEG1: u32 = 0xBFC0_0180;

impl<'ctx> Interpreter<'ctx> {
    pub fn new(bus: mem::Bus<'ctx>) -> Self {
        Self {
            bus,
            current_addr: Default::default(),
        }
    }

    /// Trigger an exception.
    fn trigger_exception(&mut self, exception: Exception) {
        eprintln!(
            "Triggered exception: {:?} @ 0x{:08X}",
            exception, self.bus.cpu.regs.pc
        );

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

        self.bus.cpu.regs.pc = exception_handler;
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

            eprintln!(
                "triggered {:?} - status: 0x{:08X} mask: 0x{:08X}",
                requested_interrupt,
                self.bus.cop0.interrupt_status.into_bits(),
                self.bus.cop0.interrupt_mask.into_bits()
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
                            _ => panic!("can't execute special op {op:?}"),
                        }
                    } else {
                        panic!("illegal special op");
                    }
                }
                _ => {
                    println!();
                    panic!("can't execute op {op:?}")
                }
            }
        } else {
            panic!("illegal op");
        }
    }

    pub fn cycle(&mut self) {
        let pc_addr = Address(self.bus.cpu.regs.pc);
        let fetched = self.bus.read(pc_addr).expect("pc is aligned");

        let (instr, instr_addr) = std::mem::replace(
            &mut self.bus.cpu.to_exec,
            (Instruction::from_bits(fetched), pc_addr),
        );
        self.current_addr = instr_addr;

        // println!("{instr_addr}  {instr}");

        if instr_addr.value() == 0x8003_0000 {
            panic!("can sideload !! :)");
        }

        if instr_addr.value() == 0xB0 {
            let call = self.bus.cpu.regs.read(Reg::R9);
            if call == 0x3D {
                let char = self.bus.cpu.regs.read(Reg::A0);
                if let Ok(char) = char::try_from(char) {
                    print!("{char}");
                }
            }
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

            let mut memory = $crate::mem::Memory::with_bios(vec![]).unwrap();
            let mut bus = $crate::mem::Bus {
                memory: &mut memory,
                cpu: &mut cpu,
                cop0: &mut cop0
            };

            for (i, byte) in code.into_iter().flat_map(|i| i.into_bits().to_le_bytes()).enumerate() {
                bus.write($crate::mem::Address(bus.cpu.regs.pc.wrapping_add(i as u32)), byte).unwrap();
            }

            #[allow(unused_mut)]
            let mut $interpreter = $crate::cpu::Interpreter::new(bus);
        };
    }

    pub(crate) use test_interpreter;
}
