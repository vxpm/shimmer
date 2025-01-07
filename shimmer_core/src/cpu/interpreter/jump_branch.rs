use super::{DEFAULT_CYCLE_COUNT, Interpreter};
use crate::cpu::{
    Reg,
    instr::{BZKind, Instruction},
};

impl Interpreter<'_> {
    /// `pc = (pc & (0b1111 << 28)) | (imm26 << 2)`
    pub fn jmp(&mut self, instr: Instruction) -> u64 {
        let high = self.bus.cpu.instr_delay_slot.1.value() & (0b1111 << 28);
        let low = instr.imm26().value() << 2;
        self.bus.cpu.regs.pc = high | low;

        DEFAULT_CYCLE_COUNT
    }

    #[inline(always)]
    fn branch(&mut self, offset: i16) {
        let addr = self
            .bus
            .cpu
            .instr_delay_slot
            .1
            .value()
            .wrapping_add_signed(i32::from(offset << 2));

        self.bus.cpu.regs.pc = addr;
    }

    /// `if rs != rt { branch(signed_imm16 << 2) }`
    pub fn bne(&mut self, instr: Instruction) -> u64 {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let rt = self.bus.cpu.regs.read(instr.rt());

        if rs != rt {
            self.branch(instr.signed_imm16());
        }

        DEFAULT_CYCLE_COUNT
    }

    /// `r31 = delay_slot + 4; pc = (pc & (0b1111 << 28)) | (imm26 << 2)`
    pub fn jal(&mut self, instr: Instruction) -> u64 {
        let high = self.bus.cpu.instr_delay_slot.1.value() & (0b1111 << 28);
        let low = instr.imm26().value() << 2;
        let addr = high | low;

        self.bus.cpu.regs.write(Reg::RA, self.bus.cpu.regs.pc);
        self.bus.cpu.regs.pc = addr;

        DEFAULT_CYCLE_COUNT
    }

    /// `pc = rs`
    pub fn jr(&mut self, instr: Instruction) -> u64 {
        let rs = self.bus.cpu.regs.read(instr.rs());
        self.bus.cpu.regs.pc = rs;

        DEFAULT_CYCLE_COUNT
    }

    /// `if rs == rt { branch(signed_imm16 << 2) }`
    pub fn beq(&mut self, instr: Instruction) -> u64 {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let rt = self.bus.cpu.regs.read(instr.rt());

        if rs == rt {
            self.branch(instr.signed_imm16());
        }

        DEFAULT_CYCLE_COUNT
    }

    /// `rd = delay_slot + 4; pc = rs`
    pub fn jalr(&mut self, instr: Instruction) -> u64 {
        let rs = self.bus.cpu.regs.read(instr.rs());
        self.bus.cpu.regs.write(instr.rd(), self.bus.cpu.regs.pc);

        self.bus.cpu.regs.pc = rs;

        DEFAULT_CYCLE_COUNT
    }

    /// `if rs > 0 { branch(signed_imm16 << 2) }`
    pub fn bgtz(&mut self, instr: Instruction) -> u64 {
        let rs = self.bus.cpu.regs.read(instr.rs()) as i32;
        if rs > 0 {
            self.branch(instr.signed_imm16());
        }

        DEFAULT_CYCLE_COUNT
    }

    /// `if rs <= 0 { branch(signed_imm16 << 2) }`
    pub fn blez(&mut self, instr: Instruction) -> u64 {
        let rs = self.bus.cpu.regs.read(instr.rs()) as i32;
        if rs <= 0 {
            self.branch(instr.signed_imm16());
        }

        DEFAULT_CYCLE_COUNT
    }

    /// `if rs ??? 0 { branch(signed_imm16 << 2) }`
    pub fn bz(&mut self, instr: Instruction) -> u64 {
        let rs = self.bus.cpu.regs.read(instr.rs()) as i32;
        match instr.bz_kind() {
            BZKind::BLTZ => {
                if rs < 0 {
                    self.branch(instr.signed_imm16());
                }
            }
            BZKind::BGEZ => {
                if rs >= 0 {
                    self.branch(instr.signed_imm16());
                }
            }
            BZKind::BLTZAL => {
                self.bus.cpu.regs.write(Reg::RA, self.bus.cpu.regs.pc);
                if rs < 0 {
                    self.branch(instr.signed_imm16());
                }
            }
            BZKind::BGEZAL => {
                self.bus.cpu.regs.write(Reg::RA, self.bus.cpu.regs.pc);
                if rs >= 0 {
                    self.branch(instr.signed_imm16());
                }
            }
        }

        DEFAULT_CYCLE_COUNT
    }
}
