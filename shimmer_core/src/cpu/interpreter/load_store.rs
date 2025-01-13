use super::{DEFAULT_CYCLE_COUNT, Interpreter};
use crate::{
    cpu::{COP, RegLoad, cop0::Exception, instr::Instruction},
    mem::Address,
};

const MEMORY_CYCLE_COUNT: u64 = 7;

impl Interpreter<'_> {
    /// `[rs + signed_imm16] = rt`
    pub fn sw(&mut self, instr: Instruction) -> u64 {
        if self.psx.cop0.regs.system_status().isolate_cache() {
            return DEFAULT_CYCLE_COUNT;
        }

        let rt = self.psx.cpu.regs.read(instr.rt());
        let rs = self.psx.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if self.psx.write::<u32, false>(addr, rt).is_err() {
            self.trigger_exception(Exception::AddressErrorStore);
        }

        MEMORY_CYCLE_COUNT
    }

    /// `rt = [rs + signed_imm16] `. Delayed by one instruction.
    pub fn lw(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if let Ok(value) = self.psx.read::<u32, false>(addr) {
            self.psx.cpu.load_delay_slot = Some(RegLoad {
                reg: instr.rt(),
                value,
            });

            if self.pending_load.is_some_and(|load| load.reg == instr.rt()) {
                self.pending_load = None;
            }
        } else {
            self.trigger_exception(Exception::AddressErrorLoad);
        }

        MEMORY_CYCLE_COUNT
    }

    /// `(half)[rs + signed_imm16] = rt`
    pub fn sh(&mut self, instr: Instruction) -> u64 {
        if self.psx.cop0.regs.system_status().isolate_cache() {
            return DEFAULT_CYCLE_COUNT;
        }

        let rt = self.psx.cpu.regs.read(instr.rt());
        let rs = self.psx.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if self.psx.write::<u16, false>(addr, rt as u16).is_err() {
            self.trigger_exception(Exception::AddressErrorStore);
        }

        MEMORY_CYCLE_COUNT
    }

    /// `(byte)[rs + signed_imm16] = rt`
    pub fn sb(&mut self, instr: Instruction) -> u64 {
        if self.psx.cop0.regs.system_status().isolate_cache() {
            return DEFAULT_CYCLE_COUNT;
        }

        let rt = self.psx.cpu.regs.read(instr.rt());
        let rs = self.psx.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if self.psx.write::<u8, false>(addr, rt as u8).is_err() {
            self.trigger_exception(Exception::AddressErrorStore);
        }

        MEMORY_CYCLE_COUNT
    }

    /// `rt = (signext)(byte)[rs + signed_imm16] `. Delayed by one instruction.
    pub fn lb(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if let Ok(value) = self.psx.read::<i8, false>(addr) {
            self.psx.cpu.load_delay_slot = Some(RegLoad {
                reg: instr.rt(),
                value: i32::from(value) as u32,
            });

            if self.pending_load.is_some_and(|load| load.reg == instr.rt()) {
                self.pending_load = None;
            }
        } else {
            self.trigger_exception(Exception::AddressErrorLoad);
        }

        MEMORY_CYCLE_COUNT
    }

    /// `rt = (zeroext)(byte)[rs + signed_imm16] `. Delayed by one instruction.
    pub fn lbu(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if let Ok(value) = self.psx.read::<u8, false>(addr) {
            self.psx.cpu.load_delay_slot = Some(RegLoad {
                reg: instr.rt(),
                value: u32::from(value),
            });

            if self.pending_load.is_some_and(|load| load.reg == instr.rt()) {
                self.pending_load = None;
            }
        } else {
            self.trigger_exception(Exception::AddressErrorLoad);
        }

        MEMORY_CYCLE_COUNT
    }

    /// `rt = (zeroext)(half)[rs + signed_imm16] `. Delayed by one instruction.
    pub fn lhu(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if let Ok(value) = self.psx.read::<u16, false>(addr) {
            self.psx.cpu.load_delay_slot = Some(RegLoad {
                reg: instr.rt(),
                value: u32::from(value),
            });

            if self.pending_load.is_some_and(|load| load.reg == instr.rt()) {
                self.pending_load = None;
            }
        } else {
            self.trigger_exception(Exception::AddressErrorLoad);
        }

        MEMORY_CYCLE_COUNT
    }

    /// `rt = (signext)(half)[rs + signed_imm16] `. Delayed by one instruction.
    pub fn lh(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if let Ok(value) = self.psx.read::<i16, false>(addr) {
            self.psx.cpu.load_delay_slot = Some(RegLoad {
                reg: instr.rt(),
                value: i32::from(value) as u32,
            });

            if self.pending_load.is_some_and(|load| load.reg == instr.rt()) {
                self.pending_load = None;
            }
        } else {
            self.trigger_exception(Exception::AddressErrorLoad);
        }

        MEMORY_CYCLE_COUNT
    }

    /// `rd = LO`.
    pub fn mflo(&mut self, instr: Instruction) -> u64 {
        self.psx.cpu.regs.write(instr.rd(), self.psx.cpu.regs.lo);
        DEFAULT_CYCLE_COUNT
    }

    /// `rd = HI`.
    pub fn mfhi(&mut self, instr: Instruction) -> u64 {
        self.psx.cpu.regs.write(instr.rd(), self.psx.cpu.regs.hi);
        DEFAULT_CYCLE_COUNT
    }

    /// `HI = rs`.
    pub fn mthi(&mut self, instr: Instruction) -> u64 {
        self.psx.cpu.regs.hi = self.psx.cpu.regs.read(instr.rs());
        DEFAULT_CYCLE_COUNT
    }

    /// `LO = rs`.
    pub fn mtlo(&mut self, instr: Instruction) -> u64 {
        self.psx.cpu.regs.lo = self.psx.cpu.regs.read(instr.rs());
        DEFAULT_CYCLE_COUNT
    }

    pub fn lwl(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let rt = if let Some(load) = self.pending_load
            && load.reg == instr.rt()
        {
            load.value
        } else {
            self.psx.cpu.regs.read(instr.rt())
        };

        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));
        let len = addr.value() % 4 + 1;

        let mut result = rt.to_be_bytes();
        for (i, byte) in (0..len).zip(result.iter_mut()) {
            let addr = addr - i;
            *byte = self.psx.read_unaligned::<u8, false>(addr);
        }

        self.psx.cpu.load_delay_slot = Some(RegLoad {
            reg: instr.rt(),
            value: u32::from_be_bytes(result),
        });

        if self.pending_load.is_some_and(|load| load.reg == instr.rt()) {
            self.pending_load = None;
        }

        MEMORY_CYCLE_COUNT
    }

    pub fn lwr(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let rt = if let Some(load) = self.pending_load
            && load.reg == instr.rt()
        {
            load.value
        } else {
            self.psx.cpu.regs.read(instr.rt())
        };

        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));
        let len = 4 - addr.value() % 4;

        let mut result = rt.to_le_bytes();
        for (i, byte) in (0..len).zip(result.iter_mut()) {
            let addr = addr + i;
            *byte = self.psx.read_unaligned::<u8, false>(addr);
        }

        self.psx.cpu.load_delay_slot = Some(RegLoad {
            reg: instr.rt(),
            value: u32::from_le_bytes(result),
        });

        if self.pending_load.is_some_and(|load| load.reg == instr.rt()) {
            self.pending_load = None;
        }

        MEMORY_CYCLE_COUNT
    }

    pub fn swl(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));
        let len = addr.value() % 4 + 1;

        let value = self.psx.cpu.regs.read(instr.rt()).to_be_bytes();
        for (i, byte) in (0..len).zip(value.iter()) {
            let addr = addr - i;
            self.psx.write_unaligned::<u8, false>(addr, *byte);
        }

        MEMORY_CYCLE_COUNT
    }

    pub fn swr(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));
        let len = 4 - addr.value() % 4;

        let value = self.psx.cpu.regs.read(instr.rt()).to_le_bytes();
        for (i, byte) in (0..len).zip(value.iter()) {
            let addr = addr + i;
            self.psx.write_unaligned::<u8, false>(addr, *byte);
        }

        MEMORY_CYCLE_COUNT
    }

    pub fn swc(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let _addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));
        let system_status = self.psx.cop0.regs.system_status();

        match instr.cop() {
            COP::COP0 if system_status.cop0_enabled_in_user_mode() => (),
            COP::COP1 if system_status.cop1_enabled() => (),
            COP::COP2 if system_status.cop2_enabled() => {
                /* let rt = self.psx.gte.regs.read(instr.rt());
                if self.psx.write(addr, rt).is_err() {
                    self.trigger_exception(Exception::AddressErrorStore);
                }*/
            }
            COP::COP3 if system_status.cop3_enabled() => (),
            _ => self.trigger_exception(Exception::CopUnusable),
        }

        MEMORY_CYCLE_COUNT
    }
}
