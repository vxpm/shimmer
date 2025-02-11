use super::{DEFAULT_DELAY, Interpreter};
use crate::cpu::Reg;
use shimmer_core::cpu::instr::{BZKind, Instruction};

impl Interpreter<'_> {
    /// `pc = (pc & (0b1111 << 28)) | (imm26 << 2)`
    pub fn jmp(&mut self, instr: Instruction) -> u64 {
        let high = self.psx.cpu.instr_delay_slot.1.value() & (0b1111 << 28);
        let low = instr.imm26().value() << 2;
        self.psx.cpu.regs.write_pc(high | low);

        DEFAULT_DELAY
    }

    #[inline(always)]
    fn branch(&mut self, offset: i16) {
        let addr = self
            .psx
            .cpu
            .instr_delay_slot
            .1
            .value()
            .wrapping_add_signed(i32::from(offset << 2));

        self.psx.cpu.regs.write_pc(addr);
    }

    /// `if rs != rt { branch(signed_imm16 << 2) }`
    pub fn bne(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let rt = self.psx.cpu.regs.read(instr.rt());

        if rs != rt {
            self.branch(instr.signed_imm16());
        }

        DEFAULT_DELAY
    }

    /// `r31 = delay_slot + 4; pc = (pc & (0b1111 << 28)) | (imm26 << 2)`
    pub fn jal(&mut self, instr: Instruction) -> u64 {
        let high = self.psx.cpu.instr_delay_slot.1.value() & (0b1111 << 28);
        let low = instr.imm26().value() << 2;
        let addr = high | low;

        self.psx
            .cpu
            .regs
            .write(Reg::RA, self.psx.cpu.regs.read_pc());
        self.cancel_load(Reg::RA);

        self.psx.cpu.regs.write_pc(addr);

        DEFAULT_DELAY
    }

    /// `pc = rs`
    pub fn jr(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        self.psx.cpu.regs.write_pc(rs);

        DEFAULT_DELAY
    }

    /// `if rs == rt { branch(signed_imm16 << 2) }`
    pub fn beq(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let rt = self.psx.cpu.regs.read(instr.rt());

        if rs == rt {
            self.branch(instr.signed_imm16());
        }

        DEFAULT_DELAY
    }

    /// `rd = delay_slot + 4; pc = rs`
    pub fn jalr(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        self.psx
            .cpu
            .regs
            .write(instr.rd(), self.psx.cpu.regs.read_pc());
        self.cancel_load(instr.rd());

        self.psx.cpu.regs.write_pc(rs);

        DEFAULT_DELAY
    }

    /// `if rs > 0 { branch(signed_imm16 << 2) }`
    pub fn bgtz(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs()) as i32;
        if rs > 0 {
            self.branch(instr.signed_imm16());
        }

        DEFAULT_DELAY
    }

    /// `if rs <= 0 { branch(signed_imm16 << 2) }`
    pub fn blez(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs()) as i32;
        if rs <= 0 {
            self.branch(instr.signed_imm16());
        }

        DEFAULT_DELAY
    }

    /// `if rs ??? 0 { branch(signed_imm16 << 2) }`
    pub fn bz(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs()) as i32;
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
                self.psx
                    .cpu
                    .regs
                    .write(Reg::RA, self.psx.cpu.regs.read_pc());
                if rs < 0 {
                    self.branch(instr.signed_imm16());
                }
            }
            BZKind::BGEZAL => {
                self.psx
                    .cpu
                    .regs
                    .write(Reg::RA, self.psx.cpu.regs.read_pc());
                if rs >= 0 {
                    self.branch(instr.signed_imm16());
                }
            }
        }

        DEFAULT_DELAY
    }
}
