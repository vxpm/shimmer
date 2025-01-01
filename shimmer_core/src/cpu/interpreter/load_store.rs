use super::Interpreter;
use crate::{cpu::instr::Instruction, mem::Address};
use tinylog::error;

impl Interpreter<'_> {
    /// `[rs + signed_imm16] = rt`
    pub fn sw(&mut self, instr: Instruction) {
        if self.bus.cop0.regs.system_status().isolate_cache() {
            return;
        }

        let rt = self.bus.cpu.regs.read(instr.rt());
        let rs = self.bus.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if self.bus.write::<u32, false>(addr, rt).is_err() {
            error!(self.bus.loggers.cpu, "sw failed on a misaligned address: {addr}"; address = addr);
        }
    }

    /// `rt = [rs + signed_imm16] `. Delayed by one instruction.
    pub fn lw(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));
        let value = self.bus.read::<u32, false>(addr).expect("aligned");

        self.bus.cpu.to_load = Some((instr.rt(), value));
    }

    /// `(half)[rs + signed_imm16] = rt`
    pub fn sh(&mut self, instr: Instruction) {
        if self.bus.cop0.regs.system_status().isolate_cache() {
            return;
        }

        let rt = self.bus.cpu.regs.read(instr.rt());
        let rs = self.bus.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if self.bus.write::<u16, false>(addr, rt as u16).is_err() {
            error!(self.bus.loggers.cpu, "sh failed on a misaligned address: {addr}"; address = addr);
        }
    }

    /// `(byte)[rs + signed_imm16] = rt`
    pub fn sb(&mut self, instr: Instruction) {
        if self.bus.cop0.regs.system_status().isolate_cache() {
            return;
        }

        let rt = self.bus.cpu.regs.read(instr.rt());
        let rs = self.bus.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if self.bus.write::<u8, false>(addr, rt as u8).is_err() {
            error!(self.bus.loggers.cpu, "sb failed on a misaligned address: {addr}"; address = addr);
        }
    }

    /// `rt = (signext)(byte)[rs + signed_imm16] `. Delayed by one instruction.
    pub fn lb(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if let Ok(value) = self.bus.read::<i8, false>(addr) {
            self.bus.cpu.to_load = Some((instr.rt(), i32::from(value) as u32));
        } else {
            error!(self.bus.loggers.cpu, "lb failed on a misaligned address: {addr}"; address = addr);
        }
    }

    /// `rt = (zeroext)(byte)[rs + signed_imm16] `. Delayed by one instruction.
    pub fn lbu(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if let Ok(value) = self.bus.read::<u8, false>(addr) {
            self.bus.cpu.to_load = Some((instr.rt(), u32::from(value)));
        } else {
            error!(self.bus.loggers.cpu, "lbu failed on a misaligned address: {addr}"; address = addr);
        }
    }

    /// `rt = (zeroext)(half)[rs + signed_imm16] `. Delayed by one instruction.
    pub fn lhu(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if let Ok(value) = self.bus.read::<u16, false>(addr) {
            self.bus.cpu.to_load = Some((instr.rt(), u32::from(value)));
        } else {
            error!(self.bus.loggers.cpu, "lhu failed on a misaligned address: {addr}"; address = addr);
        }
    }

    /// `rd = LO`.
    pub fn mflo(&mut self, instr: Instruction) {
        self.bus.cpu.regs.write(instr.rd(), self.bus.cpu.regs.lo);
    }

    /// `rd = HI`.
    pub fn mfhi(&mut self, instr: Instruction) {
        self.bus.cpu.regs.write(instr.rd(), self.bus.cpu.regs.hi);
    }

    /// `HI = rs`.
    pub fn mthi(&mut self, instr: Instruction) {
        self.bus.cpu.regs.hi = self.bus.cpu.regs.read(instr.rs());
    }

    /// `LO = rs`.
    pub fn mtlo(&mut self, instr: Instruction) {
        self.bus.cpu.regs.lo = self.bus.cpu.regs.read(instr.rs());
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        cpu::{
            Reg,
            interpreter::test::{TestState, any_reg, any_writable_reg, state, test_interpreter},
        },
        mem::{Address, Region},
    };
    use proptest::prelude::*;

    fn base_and_offset() -> impl Strategy<Value = (u32, i16)> {
        (any::<u32>(), any::<i16>().prop_map(|o| o.wrapping_mul(4))).prop_filter(
            "base and offset must add to a valid address",
            |(base, offset)| {
                let addr = Address(base.wrapping_add_signed(*offset as i32));
                let addr_end = addr + 4u32;

                addr.value() % 4 == 0
                    && addr
                        .physical()
                        .is_some_and(|p| p.region().is_some_and(|r| r != Region::IOPorts))
                    && addr_end.physical().is_some_and(|p| p.region().is_some())
            },
        )
    }

    fn store_args() -> impl Strategy<Value = (TestState, Reg, Reg, i16)> {
        (any_writable_reg(), base_and_offset()).prop_flat_map(|(rs, (base, offset))| {
            state().prop_flat_map(move |(mut cpu_regs, cop0_regs)| {
                cpu_regs.write(rs, base);
                (
                    Just((cpu_regs, cop0_regs)),
                    any_reg(),
                    Just(rs),
                    Just(offset),
                )
            })
        })
    }

    fn load_args() -> impl Strategy<Value = (TestState, Reg, Reg, i16, u32)> {
        (any_writable_reg(), base_and_offset()).prop_flat_map(|(rs, (base, offset))| {
            state().prop_flat_map(move |(mut cpu_regs, cop0_regs)| {
                cpu_regs.write(rs, base);
                (
                    Just((cpu_regs, cop0_regs)),
                    any_writable_reg(),
                    Just(rs),
                    Just(offset),
                    any::<u32>(),
                )
            })
        })
    }

    proptest::proptest! {
        #[test]
        fn sw((state, rt, rs, imm) in store_args()) {
            test_interpreter! {
                interpreter(state) =>
                sw(rt, rs, imm)
            };

            let rs = interpreter.bus.cpu.regs.read(rs);
            let rt = interpreter.bus.cpu.regs.read(rt);
            let addr = Address(rs.wrapping_add_signed(imm as i32));
            let old = interpreter.bus.read::<u32, false>(addr).unwrap();

            interpreter.cycle_n(2);

            if interpreter.bus.cop0.regs.system_status().isolate_cache() {
                prop_assert_eq!(old, interpreter.bus.read::<_, false>(addr).unwrap());
            } else {
                prop_assert_eq!(rt, interpreter.bus.read::<_, false>(addr).unwrap());
            }
        }

        #[test]
        fn lw((state, rt, rs, imm, mem_value) in load_args()) {
            test_interpreter! {
                interpreter(state) =>
                lw(rt, rs, imm)
            };

            let rs = interpreter.bus.cpu.regs.read(rs);
            let rt_start = interpreter.bus.cpu.regs.read(rt);

            // setup value at address
            let addr = Address(rs.wrapping_add_signed(imm as i32));
            interpreter.bus.write::<_, false>(addr, mem_value).unwrap();

            interpreter.cycle_n(2);

            // nothing should have changed yet: load delay
            let rt_delay = interpreter.bus.cpu.regs.read(rt);
            prop_assert_eq!(rt_delay, rt_start);

            interpreter.cycle();

            // now the value should have been loaded
            let rt = interpreter.bus.cpu.regs.read(rt);
            prop_assert_eq!(rt, interpreter.bus.read::<_, false>(addr).unwrap());
        }

        #[test]
        fn sh((state, rt, rs, imm) in store_args()) {
            test_interpreter! {
                interpreter(state) =>
                sh(rt, rs, imm)
            };

            let rs = interpreter.bus.cpu.regs.read(rs);
            let rt = interpreter.bus.cpu.regs.read(rt);
            let addr = Address(rs.wrapping_add_signed(imm as i32));
            let old = interpreter.bus.read::<u16, false>(addr).unwrap();

            interpreter.cycle_n(2);

            if interpreter.bus.cop0.regs.system_status().isolate_cache() {
                prop_assert_eq!(old, interpreter.bus.read::<_, false>(addr).unwrap());
            } else {
                prop_assert_eq!(rt as u16, interpreter.bus.read::<_, false>(addr).unwrap());
            }
        }

        #[test]
        fn sb((state, rt, rs, imm) in store_args()) {
            test_interpreter! {
                interpreter(state) =>
                sb(rt, rs, imm)
            };

            let rs = interpreter.bus.cpu.regs.read(rs);
            let rt = interpreter.bus.cpu.regs.read(rt);
            let addr = Address(rs.wrapping_add_signed(imm as i32));
            let old = interpreter.bus.read::<u8, false>(addr).unwrap();

            interpreter.cycle_n(2);

            if interpreter.bus.cop0.regs.system_status().isolate_cache() {
                prop_assert_eq!(old, interpreter.bus.read::<_, false>(addr).unwrap());
            } else {
                prop_assert_eq!(rt as u8, interpreter.bus.read::<_, false>(addr).unwrap());
            }
        }

        #[test]
        fn lb((state, rt, rs, imm, mem_value) in load_args()) {
            test_interpreter! {
                interpreter(state) =>
                lb(rt, rs, imm)
            };

            let rs = interpreter.bus.cpu.regs.read(rs);
            let rt_start = interpreter.bus.cpu.regs.read(rt);

            // setup value at address
            let addr = Address(rs.wrapping_add_signed(imm as i32));
            interpreter.bus.write::<_, false>(addr, mem_value as u8).unwrap();

            interpreter.cycle_n(2);

            // nothing should have changed yet: load delay
            let rt_delay = interpreter.bus.cpu.regs.read(rt);
            prop_assert_eq!(rt_delay, rt_start);

            interpreter.cycle();

            // now the value should have been loaded
            let rt = interpreter.bus.cpu.regs.read(rt);
            prop_assert_eq!(rt as u8, interpreter.bus.read::<_, false>(addr).unwrap());
        }
    }
}
