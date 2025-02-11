use super::{DEFAULT_DELAY, Interpreter, MEMORY_OP_DELAY};
use shimmer_core::{
    cpu::{COP, RegLoad, cop0::Exception, instr::Instruction},
    mem::Address,
};

impl Interpreter<'_> {
    /// `[rs + signed_imm16] = rt`
    pub fn sw(&mut self, instr: Instruction) -> u64 {
        if self.psx.cop0.regs.system_status().isolate_cache() {
            return DEFAULT_DELAY;
        }

        let rt = self.psx.cpu.regs.read(instr.rt());
        let rs = self.psx.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if self.psx.write::<u32, false>(addr, rt).is_err() {
            self.trigger_exception(Exception::AddressErrorStore);
        }

        MEMORY_OP_DELAY
    }

    /// `rt = [rs + signed_imm16] `. Delayed by one instruction.
    pub fn lw(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if let Ok(value) = self.psx.read::<u32, false>(addr) {
            self.cancel_load(instr.rt());
            self.psx.cpu.load_delay_slot = Some(RegLoad {
                reg: instr.rt(),
                value,
            });
        } else {
            self.trigger_exception(Exception::AddressErrorLoad);
        }

        MEMORY_OP_DELAY
    }

    /// `(half)[rs + signed_imm16] = rt`
    pub fn sh(&mut self, instr: Instruction) -> u64 {
        if self.psx.cop0.regs.system_status().isolate_cache() {
            return DEFAULT_DELAY;
        }

        let rt = self.psx.cpu.regs.read(instr.rt());
        let rs = self.psx.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if self.psx.write::<u16, false>(addr, rt as u16).is_err() {
            self.trigger_exception(Exception::AddressErrorStore);
        }

        MEMORY_OP_DELAY
    }

    /// `(byte)[rs + signed_imm16] = rt`
    pub fn sb(&mut self, instr: Instruction) -> u64 {
        if self.psx.cop0.regs.system_status().isolate_cache() {
            return DEFAULT_DELAY;
        }

        let rt = self.psx.cpu.regs.read(instr.rt());
        let rs = self.psx.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if self.psx.write::<u8, false>(addr, rt as u8).is_err() {
            self.trigger_exception(Exception::AddressErrorStore);
        }

        MEMORY_OP_DELAY
    }

    /// `rt = (signext)(byte)[rs + signed_imm16] `. Delayed by one instruction.
    pub fn lb(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if let Ok(value) = self.psx.read::<i8, false>(addr) {
            self.cancel_load(instr.rt());
            self.psx.cpu.load_delay_slot = Some(RegLoad {
                reg: instr.rt(),
                value: i32::from(value) as u32,
            });
        } else {
            self.trigger_exception(Exception::AddressErrorLoad);
        }

        MEMORY_OP_DELAY
    }

    /// `rt = (zeroext)(byte)[rs + signed_imm16] `. Delayed by one instruction.
    pub fn lbu(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if let Ok(value) = self.psx.read::<u8, false>(addr) {
            self.cancel_load(instr.rt());
            self.psx.cpu.load_delay_slot = Some(RegLoad {
                reg: instr.rt(),
                value: u32::from(value),
            });
        } else {
            self.trigger_exception(Exception::AddressErrorLoad);
        }

        MEMORY_OP_DELAY
    }

    /// `rt = (zeroext)(half)[rs + signed_imm16] `. Delayed by one instruction.
    pub fn lhu(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if let Ok(value) = self.psx.read::<u16, false>(addr) {
            self.cancel_load(instr.rt());
            self.psx.cpu.load_delay_slot = Some(RegLoad {
                reg: instr.rt(),
                value: u32::from(value),
            });
        } else {
            self.trigger_exception(Exception::AddressErrorLoad);
        }

        MEMORY_OP_DELAY
    }

    /// `rt = (signext)(half)[rs + signed_imm16] `. Delayed by one instruction.
    pub fn lh(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let addr = Address(rs.wrapping_add_signed(i32::from(instr.signed_imm16())));

        if let Ok(value) = self.psx.read::<i16, false>(addr) {
            self.cancel_load(instr.rt());
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

        MEMORY_OP_DELAY
    }

    /// `rd = LO`.
    pub fn mflo(&mut self, instr: Instruction) -> u64 {
        self.cancel_load(instr.rd());
        self.psx
            .cpu
            .regs
            .write(instr.rd(), self.psx.cpu.regs.read_lo());
        DEFAULT_DELAY
    }

    /// `rd = HI`.
    pub fn mfhi(&mut self, instr: Instruction) -> u64 {
        self.cancel_load(instr.rd());
        self.psx
            .cpu
            .regs
            .write(instr.rd(), self.psx.cpu.regs.read_hi());
        DEFAULT_DELAY
    }

    /// `HI = rs`.
    pub fn mthi(&mut self, instr: Instruction) -> u64 {
        self.psx
            .cpu
            .regs
            .write_hi(self.psx.cpu.regs.read(instr.rs()));
        DEFAULT_DELAY
    }

    /// `LO = rs`.
    pub fn mtlo(&mut self, instr: Instruction) -> u64 {
        self.psx
            .cpu
            .regs
            .write_lo(self.psx.cpu.regs.read(instr.rs()));
        DEFAULT_DELAY
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

        self.cancel_load(instr.rt());
        self.psx.cpu.load_delay_slot = Some(RegLoad {
            reg: instr.rt(),
            value: u32::from_be_bytes(result),
        });

        MEMORY_OP_DELAY
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

        self.cancel_load(instr.rt());
        self.psx.cpu.load_delay_slot = Some(RegLoad {
            reg: instr.rt(),
            value: u32::from_le_bytes(result),
        });

        MEMORY_OP_DELAY
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

        MEMORY_OP_DELAY
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

        MEMORY_OP_DELAY
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

        MEMORY_OP_DELAY
    }
}
