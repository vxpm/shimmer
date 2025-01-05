use super::Interpreter;
use crate::{
    cpu::{cop0::Exception, instr::Instruction},
    mem::Address,
};

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
            self.trigger_exception(Exception::AddressErrorStore);
        }
    }

    /// `rt = [rs + signed_imm16] `. Delayed by one instruction.
    pub fn lw(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if let Ok(value) = self.bus.read::<u32, false>(addr) {
            self.bus.cpu.load_delay_slot = Some((instr.rt(), value));
        } else {
            self.trigger_exception(Exception::AddressErrorLoad);
        }
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
            self.trigger_exception(Exception::AddressErrorStore);
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
            self.trigger_exception(Exception::AddressErrorStore);
        }
    }

    /// `rt = (signext)(byte)[rs + signed_imm16] `. Delayed by one instruction.
    pub fn lb(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if let Ok(value) = self.bus.read::<i8, false>(addr) {
            self.bus.cpu.load_delay_slot = Some((instr.rt(), i32::from(value) as u32));
        } else {
            self.trigger_exception(Exception::AddressErrorLoad);
        }
    }

    /// `rt = (zeroext)(byte)[rs + signed_imm16] `. Delayed by one instruction.
    pub fn lbu(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if let Ok(value) = self.bus.read::<u8, false>(addr) {
            self.bus.cpu.load_delay_slot = Some((instr.rt(), u32::from(value)));
        } else {
            self.trigger_exception(Exception::AddressErrorLoad);
        }
    }

    /// `rt = (zeroext)(half)[rs + signed_imm16] `. Delayed by one instruction.
    pub fn lhu(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if let Ok(value) = self.bus.read::<u16, false>(addr) {
            self.bus.cpu.load_delay_slot = Some((instr.rt(), u32::from(value)));
        } else {
            self.trigger_exception(Exception::AddressErrorLoad);
        }
    }

    /// `rt = (signext)(half)[rs + signed_imm16] `. Delayed by one instruction.
    pub fn lh(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if let Ok(value) = self.bus.read::<i16, false>(addr) {
            self.bus.cpu.load_delay_slot = Some((instr.rt(), i32::from(value) as u32));
        } else {
            self.trigger_exception(Exception::AddressErrorLoad);
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

    pub fn lwl(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));
        let len = addr.value() % 4 + 1;

        let mut result = self.bus.cpu.regs.read(instr.rt()).to_be_bytes();
        for (i, byte) in (0..len).zip(result.iter_mut()) {
            let addr = addr - i;
            *byte = self.bus.read_unaligned::<u8, false>(addr);
        }

        self.bus
            .cpu
            .regs
            .write(instr.rt(), u32::from_be_bytes(result));
    }

    pub fn lwr(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));
        let len = 4 - addr.value() % 4;

        let mut result = self.bus.cpu.regs.read(instr.rt()).to_le_bytes();
        for (i, byte) in (0..len).zip(result.iter_mut()) {
            let addr = addr + i;
            *byte = self.bus.read_unaligned::<u8, false>(addr);
        }

        self.bus
            .cpu
            .regs
            .write(instr.rt(), u32::from_le_bytes(result));
    }

    pub fn swl(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));
        let len = addr.value() % 4 + 1;

        let value = self.bus.cpu.regs.read(instr.rt()).to_be_bytes();
        for (i, byte) in (0..len).zip(value.iter()) {
            let addr = addr - i;
            self.bus.write_unaligned::<u8, false>(addr, *byte);
        }
    }

    pub fn swr(&mut self, instr: Instruction) {
        let rs = self.bus.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));
        let len = 4 - addr.value() % 4;

        let value = self.bus.cpu.regs.read(instr.rt()).to_le_bytes();
        for (i, byte) in (0..len).zip(value.iter()) {
            let addr = addr + i;
            self.bus.write_unaligned::<u8, false>(addr, *byte);
        }
    }
}
