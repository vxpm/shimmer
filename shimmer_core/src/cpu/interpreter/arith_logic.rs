use super::Interpreter;
use crate::cpu::{cop0::Exception, instr::Instruction};

impl Interpreter<'_> {
    /// `rt = (imm16 << 16)`
    pub fn lui(&mut self, instr: Instruction) {
        let result = u32::from(instr.imm16()) << 16;
        self.bus.cpu.regs.write(instr.rt(), result);
    }

    /// `rt = rs | imm16`
    pub fn ori(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let result = rs | u32::from(instr.imm16());
        self.bus.cpu.regs.write(instr.rt(), result);
    }

    /// `rd = rt << imm5`
    pub fn sll(&mut self, instr: Instruction) {
        let rt = self.bus.cpu.regs.read(instr.rt());
        let result = rt.unbounded_shl(u32::from(instr.imm5().value()));
        self.bus.cpu.regs.write(instr.rd(), result);
    }

    /// `rt = rs + imm16`
    pub fn addiu(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let result = rs.wrapping_add_signed(i32::from(instr.signed_imm16()));
        self.bus.cpu.regs.write(instr.rt(), result);
    }

    /// `rd = rs | rt`
    pub fn or(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let rt = self.bus.cpu.regs.read(instr.rt());
        let result = rs | rt;

        self.bus.cpu.regs.write(instr.rd(), result);
    }

    /// `rt = rs + signed_imm16`
    pub fn addi(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs()) as i32;

        let result = rs.checked_add(i32::from(instr.signed_imm16()));
        if let Some(value) = result {
            self.bus.cpu.regs.write(instr.rt(), value as u32);
        } else {
            self.trigger_exception(Exception::ArithmeticOverflow);
        }
    }

    /// `if rs < rt { rd = 1 } else { rd = 0 }`
    pub fn sltu(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let rt = self.bus.cpu.regs.read(instr.rt());
        self.bus.cpu.regs.write(instr.rd(), u32::from(rs < rt));
    }

    /// `rd = rs + rt`
    pub fn addu(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let rt = self.bus.cpu.regs.read(instr.rt());

        let result = rs.wrapping_add(rt);
        self.bus.cpu.regs.write(instr.rd(), result);
    }

    /// `rt = rs & imm16`
    pub fn andi(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let result = rs & u32::from(instr.imm16());
        self.bus.cpu.regs.write(instr.rt(), result);
    }

    /// `rd = rt >> imm5`
    pub fn srl(&mut self, instr: Instruction) {
        let rt = self.bus.cpu.regs.read(instr.rt());
        let result = rt.unbounded_shr(u32::from(instr.imm5().value()));
        self.bus.cpu.regs.write(instr.rd(), result);
    }

    /// `rd = rs & rt`
    pub fn and(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let rt = self.bus.cpu.regs.read(instr.rt());
        let result = rs & rt;

        self.bus.cpu.regs.write(instr.rd(), result);
    }

    /// `rd = rs + rt`
    pub fn add(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs()) as i32;
        let rt = self.bus.cpu.regs.read(instr.rt()) as i32;

        let result = rs.checked_add(rt);
        if let Some(value) = result {
            self.bus.cpu.regs.write(instr.rd(), value as u32);
        } else {
            self.trigger_exception(Exception::ArithmeticOverflow);
        }
    }

    /// `if rs < signed_imm16 { rt = 1 } else { rt = 0 }`
    pub fn slti(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs()) as i32;
        self.bus
            .cpu
            .regs
            .write(instr.rd(), u32::from(rs < i32::from(instr.signed_imm16())));
    }

    /// `rd = rs - rt`
    pub fn subu(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let rt = self.bus.cpu.regs.read(instr.rt());
        let result = rs.wrapping_sub(rt);

        self.bus.cpu.regs.write(instr.rd(), result);
    }

    /// `rd = rt (signed)>> imm5`
    pub fn sra(&mut self, instr: Instruction) {
        let rt = self.bus.cpu.regs.read(instr.rt()) as i32;
        let result = rt.unbounded_shr(u32::from(instr.imm5().value()));
        self.bus.cpu.regs.write(instr.rd(), result as u32);
    }

    /// `rd = rt (signed)>> imm5`
    pub fn div(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let rt = self.bus.cpu.regs.read(instr.rt());
        let (div, rem) = (
            rs.checked_div(rt).unwrap_or_default(),
            rs.checked_rem(rt).unwrap_or_default(),
        );

        self.bus.cpu.regs.lo = div;
        self.bus.cpu.regs.hi = rem;
    }

    /// `if rs < signed_imm16 { rt = 1 } else { rt = 0 }`
    pub fn sltiu(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        self.bus.cpu.regs.write(
            instr.rd(),
            u32::from(rs < (i32::from(instr.signed_imm16()) as u32)),
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::cpu::interpreter::test::{any_reg, any_writable_reg, state, test_interpreter};
    use bitos::integer::u5;
    use proptest::prelude::*;

    proptest::proptest! {
        #[test]
        fn lui(state in state(), rt in any_writable_reg(), imm in any::<u16>()) {
            test_interpreter! {
                interpreter(state) =>
                lui(rt, imm)
            };

            interpreter.cycle_n(2);

            let rt = interpreter.bus.cpu.regs.read(rt);
            prop_assert_eq!((rt & 0xFFFF_0000) >> 16, imm as u32);
            prop_assert_eq!(rt & 0xFFFF, 0);
        }

        #[test]
        fn ori(state in state(), rt in any_writable_reg(), rs in any_reg(), imm in any::<u16>()) {
            test_interpreter! {
                interpreter(state) =>
                ori(rt, rs, imm)
            };

            let rs = interpreter.bus.cpu.regs.read(rs);
            interpreter.cycle_n(2);

            let rt = interpreter.bus.cpu.regs.read(rt);
            prop_assert_eq!(rt, rs | imm as u32);
        }

        #[test]
        fn sll(state in state(), rd in any_writable_reg(), rt in any_reg(), imm in u5::any()) {
            test_interpreter! {
                interpreter(state) =>
                sll(rd, rt, imm)
            };

            let rt = interpreter.bus.cpu.regs.read(rt);
            interpreter.cycle_n(2);

            let rd = interpreter.bus.cpu.regs.read(rd);
            prop_assert_eq!(rd, rt.unbounded_shl(imm.value() as u32));
        }

        #[test]
        fn addiu(state in state(), rt in any_writable_reg(), rs in any_reg(), imm in any::<i16>()) {
            test_interpreter! {
                interpreter(state) =>
                addiu(rt, rs, imm)
            };

            let rs = interpreter.bus.cpu.regs.read(rs);
            interpreter.cycle_n(2);

            let rt = interpreter.bus.cpu.regs.read(rt);
            prop_assert_eq!(rt, rs.wrapping_add_signed(imm as i32));
        }

        #[test]
        fn or(state in state(), rd in any_writable_reg(), rs in any_reg(), rt in any_reg()) {
            test_interpreter! {
                interpreter(state) =>
                or(rd, rs, rt)
            };

            let rs = interpreter.bus.cpu.regs.read(rs);
            let rt = interpreter.bus.cpu.regs.read(rt);
            interpreter.cycle_n(2);

            let rd = interpreter.bus.cpu.regs.read(rd);
            prop_assert_eq!(rd, rs | rt);
        }

        #[test]
        fn addi(state in state(), rt in any_writable_reg(), rs in any_reg(), imm in any::<i16>()) {
            test_interpreter! {
                interpreter(state) =>
                addi(rt, rs, imm)
            };

            let rs = interpreter.bus.cpu.regs.read(rs) as i32;
            interpreter.cycle_n(2);

            let rt = interpreter.bus.cpu.regs.read(rt);
            prop_assert_eq!(rt, rs.wrapping_add(imm as i32) as u32);
        }

        #[test]
        fn sltu(state in state(), rd in any_writable_reg(), rs in any_reg(), rt in any_reg()) {
            test_interpreter! {
                interpreter(state) =>
                sltu(rd, rs, rt)
            };

            let rs = interpreter.bus.cpu.regs.read(rs);
            let rt = interpreter.bus.cpu.regs.read(rt);
            interpreter.cycle_n(2);

            let rd = interpreter.bus.cpu.regs.read(rd);
            prop_assert_eq!(rd, (rs < rt) as u32);
        }

        #[test]
        fn addu(state in state(), rd in any_writable_reg(), rs in any_reg(), rt in any_reg()) {
            test_interpreter! {
                interpreter(state) =>
                addu(rd, rs, rt)
            };

            let rs = interpreter.bus.cpu.regs.read(rs);
            let rt = interpreter.bus.cpu.regs.read(rt);
            interpreter.cycle_n(2);

            let rd = interpreter.bus.cpu.regs.read(rd);
            prop_assert_eq!(rd, rs.wrapping_add(rt));
        }

        #[test]
        fn andi(state in state(), rt in any_writable_reg(), rs in any_reg(), imm in any::<u16>()) {
            test_interpreter! {
                interpreter(state) =>
                andi(rt, rs, imm)
            };

            let rs = interpreter.bus.cpu.regs.read(rs);
            interpreter.cycle_n(2);

            let rt = interpreter.bus.cpu.regs.read(rt);
            prop_assert_eq!(rt, rs & imm as u32);
        }

        #[test]
        fn srl(state in state(), rd in any_writable_reg(), rt in any_reg(), imm in u5::any()) {
            test_interpreter! {
                interpreter(state) =>
                srl(rd, rt, imm)
            };

            let rt = interpreter.bus.cpu.regs.read(rt);
            interpreter.cycle_n(2);

            let rd = interpreter.bus.cpu.regs.read(rd);
            prop_assert_eq!(rd, rt.unbounded_shr(imm.value() as u32));
        }

        #[test]
        fn and(state in state(), rd in any_writable_reg(), rs in any_reg(), rt in any_reg()) {
            test_interpreter! {
                interpreter(state) =>
                and(rd, rs, rt)
            };

            let rs = interpreter.bus.cpu.regs.read(rs);
            let rt = interpreter.bus.cpu.regs.read(rt);
            interpreter.cycle_n(2);

            let rd = interpreter.bus.cpu.regs.read(rd);
            prop_assert_eq!(rd, rs & rt);
        }

        #[test]
        fn add(state in state(), rd in any_writable_reg(), rs in any_reg(), rt in any_reg()) {
            test_interpreter! {
                interpreter(state) =>
                add(rd, rs, rt)
            };

            let rs = interpreter.bus.cpu.regs.read(rs) as i32;
            let rt = interpreter.bus.cpu.regs.read(rt) as i32;
            let rd_old = interpreter.bus.cpu.regs.read(rd);
            interpreter.cycle_n(2);

            let rd = interpreter.bus.cpu.regs.read(rd);

            if let Some(value) = rs.checked_add(rt) {
                prop_assert_eq!(rd, value as u32);
            } else {
                prop_assert_eq!(rd, rd_old);
            }
        }
    }
}
