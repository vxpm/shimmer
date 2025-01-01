use super::Interpreter;
use crate::cpu::{
    Reg,
    instr::{BZKind, Instruction},
};

impl Interpreter<'_> {
    /// `pc = (pc & (0b1111 << 28)) | (imm26 << 2)`
    pub fn jmp(&mut self, instr: Instruction) {
        let high = self.bus.cpu.regs.pc & (0b1111 << 28);
        let low = instr.imm26().value() << 2;
        let addr = high | low;

        // subtract 4 to account for the increment after instruction is executed
        self.bus.cpu.regs.pc = addr.wrapping_sub(4);
    }

    pub fn branch(&mut self, offset: i16) {
        let addr = self
            .bus
            .cpu
            .regs
            .pc
            .wrapping_add_signed(i32::from(offset) << 2);

        // subtract 4 to account for the increment after instruction is executed
        self.bus.cpu.regs.pc = addr.wrapping_sub(4);
    }

    /// `if rs != rt { branch(signed_imm16 << 2) }`
    pub fn bne(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let rt = self.bus.cpu.regs.read(instr.rt());

        if rs != rt {
            self.branch(instr.signed_imm16());
        }
    }

    /// `r31 = pc + 4; pc = (pc & (0b1111 << 28)) | (imm26 << 2)`
    pub fn jal(&mut self, instr: Instruction) {
        let high = self.bus.cpu.regs.pc & (0b1111 << 28);
        let low = instr.imm26().value() << 2;
        let addr = high | low;

        self.bus
            .cpu
            .regs
            .write(Reg::RA, self.bus.cpu.regs.pc.wrapping_add(4));

        // subtract 4 to account for the increment after instruction is executed
        self.bus.cpu.regs.pc = addr.wrapping_sub(4);
    }

    /// `pc = rs`
    pub fn jr(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());

        // subtract 4 to account for the increment after instruction is executed
        self.bus.cpu.regs.pc = rs.wrapping_sub(4);
    }

    /// `if rs == rt { branch(signed_imm16 << 2) }`
    pub fn beq(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let rt = self.bus.cpu.regs.read(instr.rt());

        if rs == rt {
            self.branch(instr.signed_imm16());
        }
    }

    /// `rd = pc + 4; pc = rs`
    pub fn jalr(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        self.bus
            .cpu
            .regs
            .write(instr.rd(), self.bus.cpu.regs.pc.wrapping_add(4));

        // subtract 4 to account for the increment after instruction is executed
        self.bus.cpu.regs.pc = rs.wrapping_sub(4);
    }

    /// `if rs > 0 { branch(signed_imm16 << 2) }`
    pub fn bgtz(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs()) as i32;
        if rs > 0 {
            self.branch(instr.signed_imm16());
        }
    }

    /// `if rs <= 0 { branch(signed_imm16 << 2) }`
    pub fn blez(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs()) as i32;
        if rs <= 0 {
            self.branch(instr.signed_imm16());
        }
    }

    /// `if rs ??? 0 { branch(signed_imm16 << 2) }`
    pub fn bz(&mut self, instr: Instruction) {
        if let Some(kind) = instr.bz_kind() {
            let rs = self.bus.cpu.regs.read(instr.rs()) as i32;
            match kind {
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
                    if rs < 0 {
                        self.branch(instr.signed_imm16());
                    }

                    self.bus
                        .cpu
                        .regs
                        .write(Reg::RA, self.bus.cpu.regs.pc.wrapping_add(4));
                }
                BZKind::BGEZAL => {
                    if rs >= 0 {
                        self.branch(instr.signed_imm16());
                    }

                    self.bus
                        .cpu
                        .regs
                        .write(Reg::RA, self.bus.cpu.regs.pc.wrapping_add(4));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        cpu::{
            Reg,
            interpreter::test::{TestState, any_reg, state, test_interpreter},
        },
        mem::{Address, Region},
    };
    use bitos::integer::u26;
    use proptest::prelude::*;

    fn valid_jmp_target() -> impl Strategy<Value = u26> {
        (u26::any()).prop_filter("target must be a valid address", |addr| {
            let addr = Address(addr.value() << 2);
            addr.value() % 4 == 0
                && addr
                    .physical()
                    .is_some_and(|p| p.region().is_some_and(|r| r != Region::IOPorts))
        })
    }

    fn branch_base_and_offset() -> impl Strategy<Value = (u32, i16)> {
        (any::<u32>(), any::<i16>()).prop_filter(
            "resulting address must be a valid address",
            |(base, offset)| {
                let addr = Address(base.wrapping_add_signed(i32::from(*offset) << 2));

                let valid_addr_range = (-128..128).all(|offset| {
                    let addr = Address(base.wrapping_add_signed(offset << 2));
                    addr.physical()
                        .is_some_and(|p| p.region().is_some_and(|r| r != Region::IOPorts))
                });

                addr.value() % 4 == 0 && valid_addr_range
            },
        )
    }

    fn branch_args() -> impl Strategy<Value = (TestState, Reg, Reg, i16)> {
        branch_base_and_offset().prop_flat_map(|(base, offset)| {
            state().prop_flat_map(move |(mut cpu_regs, cop0_regs)| {
                cpu_regs.pc = base;
                (
                    Just((cpu_regs, cop0_regs)),
                    any_reg(),
                    any_reg(),
                    Just(offset),
                )
            })
        })
    }

    proptest::proptest! {
        #[test]
        fn jmp(state in state(), target in valid_jmp_target()) {
            test_interpreter! {
                interpreter(state) =>
                jmp(target)
            };

            let high = interpreter.bus.cpu.regs.pc & (0b1111 << 28);
            let low = target.value() << 2;
            let addr = high | low;
            interpreter.cycle_n(2);

            prop_assert_eq!(interpreter.bus.cpu.regs.pc, addr);
        }

        #[test]
        fn bne((state, rs, rt, offset) in branch_args()) {
            test_interpreter! {
                interpreter(state) =>
                bne(rs, rt, offset)
            };

            let rs = interpreter.bus.cpu.regs.read(rs);
            let rt = interpreter.bus.cpu.regs.read(rt);

            interpreter.cycle();
            let result = if rs != rt {
                interpreter.bus.cpu.regs.pc.wrapping_add_signed(i32::from(offset) << 2)
            } else {
                interpreter.bus.cpu.regs.pc.wrapping_add(4)
            };
            interpreter.cycle();

            prop_assert_eq!(interpreter.bus.cpu.regs.pc, result);
        }

        #[test]
        fn jal(state in state(), target in valid_jmp_target()) {
            test_interpreter! {
                interpreter(state) =>
                jal(target)
            };

            let high = interpreter.bus.cpu.regs.pc & (0b1111 << 28);
            let low = target.value() << 2;
            let addr = high | low;
            let pc = interpreter.bus.cpu.regs.pc;
            interpreter.cycle_n(2);

            prop_assert_eq!(interpreter.bus.cpu.regs.pc, addr);
            prop_assert_eq!(interpreter.bus.cpu.regs.read(Reg::RA), pc.wrapping_add(8));
        }

        #[test]
        fn beq((state, rs, rt, offset) in branch_args()) {
            test_interpreter! {
                interpreter(state) =>
                beq(rs, rt, offset)
            };

            let rs = interpreter.bus.cpu.regs.read(rs);
            let rt = interpreter.bus.cpu.regs.read(rt);

            interpreter.cycle();
            let result = if rs == rt {
                interpreter.bus.cpu.regs.pc.wrapping_add_signed(i32::from(offset) << 2)
            } else {
                interpreter.bus.cpu.regs.pc.wrapping_add(4)
            };
            interpreter.cycle();

            prop_assert_eq!(interpreter.bus.cpu.regs.pc, result);
        }
    }
}
