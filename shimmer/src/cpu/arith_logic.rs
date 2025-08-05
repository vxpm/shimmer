use super::{DEFAULT_DELAY, Interpreter};
use crate::PSX;
use shimmer_core::cpu::{cop0::Exception, instr::Instruction};

impl Interpreter {
    /// `rt = (imm16 << 16)`
    pub fn lui(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let result = u32::from(instr.imm16()) << 16;
        psx.cpu.regs.write(instr.rt(), result);
        self.cancel_load(instr.rt());

        DEFAULT_DELAY
    }

    /// `rt = rs | imm16`
    pub fn ori(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rs = psx.cpu.regs.read(instr.rs());
        let result = rs | u32::from(instr.imm16());
        psx.cpu.regs.write(instr.rt(), result);
        self.cancel_load(instr.rt());

        DEFAULT_DELAY
    }

    /// `rd = rt << imm5`
    #[inline(always)]
    pub fn sll(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rt = psx.cpu.regs.read(instr.rt());
        let result = rt.unbounded_shl(u32::from(instr.imm5().value()));
        psx.cpu.regs.write(instr.rd(), result);
        self.cancel_load(instr.rd());

        DEFAULT_DELAY
    }

    /// `rt = rs + imm16`
    #[inline(always)]
    pub fn addiu(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rs = psx.cpu.regs.read(instr.rs());
        let result = rs.wrapping_add_signed(i32::from(instr.signed_imm16()));
        psx.cpu.regs.write(instr.rt(), result);
        self.cancel_load(instr.rt());

        DEFAULT_DELAY
    }

    /// `rd = rs | rt`
    pub fn or(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rs = psx.cpu.regs.read(instr.rs());
        let rt = psx.cpu.regs.read(instr.rt());
        psx.cpu.regs.write(instr.rd(), rs | rt);
        self.cancel_load(instr.rd());

        DEFAULT_DELAY
    }

    /// `rt = rs + signed_imm16`
    pub fn addi(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rs = psx.cpu.regs.read(instr.rs()) as i32;
        let result = rs.checked_add(i32::from(instr.signed_imm16()));

        if let Some(value) = result {
            psx.cpu.regs.write(instr.rt(), value as u32);
            self.cancel_load(instr.rt());
        } else {
            self.trigger_exception(psx, Exception::ArithmeticOverflow);
        }

        DEFAULT_DELAY
    }

    /// `if rs < rt { rd = 1 } else { rd = 0 }`
    pub fn sltu(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rs = psx.cpu.regs.read(instr.rs());
        let rt = psx.cpu.regs.read(instr.rt());
        psx.cpu.regs.write(instr.rd(), u32::from(rs < rt));
        self.cancel_load(instr.rd());

        DEFAULT_DELAY
    }

    /// `rd = rs + rt`
    pub fn addu(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rs = psx.cpu.regs.read(instr.rs());
        let rt = psx.cpu.regs.read(instr.rt());
        psx.cpu.regs.write(instr.rd(), rs.wrapping_add(rt));
        self.cancel_load(instr.rd());

        DEFAULT_DELAY
    }

    /// `rt = rs & imm16`
    pub fn andi(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rs = psx.cpu.regs.read(instr.rs());
        let result = rs & u32::from(instr.imm16());
        psx.cpu.regs.write(instr.rt(), result);
        self.cancel_load(instr.rt());

        DEFAULT_DELAY
    }

    /// `rd = rt >> imm5`
    pub fn srl(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rt = psx.cpu.regs.read(instr.rt());
        let result = rt.unbounded_shr(u32::from(instr.imm5().value()));
        psx.cpu.regs.write(instr.rd(), result);
        self.cancel_load(instr.rd());

        DEFAULT_DELAY
    }

    /// `rd = rs & rt`
    pub fn and(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rs = psx.cpu.regs.read(instr.rs());
        let rt = psx.cpu.regs.read(instr.rt());
        psx.cpu.regs.write(instr.rd(), rs & rt);
        self.cancel_load(instr.rd());

        DEFAULT_DELAY
    }

    /// `rd = rs + rt`
    pub fn add(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rs = psx.cpu.regs.read(instr.rs()) as i32;
        let rt = psx.cpu.regs.read(instr.rt()) as i32;

        let result = rs.checked_add(rt);
        if let Some(value) = result {
            psx.cpu.regs.write(instr.rd(), value as u32);
            self.cancel_load(instr.rd());
        } else {
            self.trigger_exception(psx, Exception::ArithmeticOverflow);
        }

        DEFAULT_DELAY
    }

    /// `if rs < signed_imm16 { rt = 1 } else { rt = 0 }`
    pub fn slti(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rs = psx.cpu.regs.read(instr.rs()) as i32;
        let result = rs < i32::from(instr.signed_imm16());
        psx.cpu.regs.write(instr.rt(), u32::from(result));
        self.cancel_load(instr.rt());

        DEFAULT_DELAY
    }

    /// `rd = rs - rt`
    pub fn subu(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rs = psx.cpu.regs.read(instr.rs());
        let rt = psx.cpu.regs.read(instr.rt());
        psx.cpu.regs.write(instr.rd(), rs.wrapping_sub(rt));
        self.cancel_load(instr.rd());

        DEFAULT_DELAY
    }

    /// `rd = rt (signed)>> imm5`
    pub fn sra(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rt = psx.cpu.regs.read(instr.rt()) as i32;
        let result = rt.unbounded_shr(u32::from(instr.imm5().value()));
        psx.cpu.regs.write(instr.rd(), result as u32);
        self.cancel_load(instr.rd());

        DEFAULT_DELAY
    }

    /// `LO = rs (signed)/ rt; HI = rs (signed)% rt`
    pub fn div(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rs = psx.cpu.regs.read(instr.rs()) as i32;
        let rt = psx.cpu.regs.read(instr.rt()) as i32;
        let (div, rem) = match (rs, rt) {
            (0.., 0) => (-1, rs),
            (..0, 0) => (1, rs),
            (i32::MIN, -1) => (i32::MIN, 0),
            (rs, rt) => (
                rs.checked_div(rt).unwrap_or_default(),
                rs.checked_rem(rt).unwrap_or_default(),
            ),
        };

        psx.cpu.regs.write_lo(div as u32);
        psx.cpu.regs.write_hi(rem as u32);

        DEFAULT_DELAY
    }

    /// `if rs < signed_imm16 { rt = 1 } else { rt = 0 }`
    pub fn sltiu(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rs = psx.cpu.regs.read(instr.rs());
        let result = rs < (i32::from(instr.signed_imm16()) as u32);
        psx.cpu.regs.write(instr.rt(), u32::from(result));
        self.cancel_load(instr.rt());

        DEFAULT_DELAY
    }

    /// `if rs (signed)< rt { rd = 1 } else { rd = 0 }`
    pub fn slt(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rs = psx.cpu.regs.read(instr.rs()) as i32;
        let rt = psx.cpu.regs.read(instr.rt()) as i32;
        psx.cpu.regs.write(instr.rd(), u32::from(rs < rt));
        self.cancel_load(instr.rd());

        DEFAULT_DELAY
    }

    /// `LO = rs / rt; HI = rs % rt`
    pub fn divu(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rs = psx.cpu.regs.read(instr.rs());
        let rt = psx.cpu.regs.read(instr.rt());
        let (div, rem) = (
            rs.checked_div(rt).unwrap_or(!0),
            rs.checked_rem(rt).unwrap_or(rs),
        );

        psx.cpu.regs.write_lo(div);
        psx.cpu.regs.write_hi(rem);

        DEFAULT_DELAY
    }

    /// `rd = rt << (rs & 0x1F)`
    pub fn sllv(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rt = psx.cpu.regs.read(instr.rt());
        let rs = psx.cpu.regs.read(instr.rs());
        let result = rt.unbounded_shl(rs & 0x1F);
        psx.cpu.regs.write(instr.rd(), result);
        self.cancel_load(instr.rd());

        DEFAULT_DELAY
    }

    /// `rd = !(rs | rt)`
    pub fn nor(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rs = psx.cpu.regs.read(instr.rs());
        let rt = psx.cpu.regs.read(instr.rt());
        psx.cpu.regs.write(instr.rd(), !(rs | rt));
        self.cancel_load(instr.rd());

        DEFAULT_DELAY
    }

    /// `rd = rt (signed)>> (rs & 0x1F)`
    pub fn srav(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rt = psx.cpu.regs.read(instr.rt()) as i32;
        let rs = psx.cpu.regs.read(instr.rs());
        let result = rt.unbounded_shr(rs & 0x1F);
        psx.cpu.regs.write(instr.rd(), result as u32);
        self.cancel_load(instr.rd());

        DEFAULT_DELAY
    }

    /// `rd = rt >> (rs & 0x1F)`
    pub fn srlv(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rt = psx.cpu.regs.read(instr.rt());
        let rs = psx.cpu.regs.read(instr.rs());
        let result = rt.unbounded_shr(rs & 0x1F);
        psx.cpu.regs.write(instr.rd(), result);
        self.cancel_load(instr.rd());

        DEFAULT_DELAY
    }

    pub fn multu(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rs = u64::from(psx.cpu.regs.read(instr.rs()));
        let rt = u64::from(psx.cpu.regs.read(instr.rt()));
        let result = zerocopy::byteorder::little_endian::U64::new(rs * rt);
        let [low, high]: [zerocopy::byteorder::little_endian::U32; 2] =
            zerocopy::transmute!(result);

        psx.cpu.regs.write_lo(low.get());
        psx.cpu.regs.write_hi(high.get());

        DEFAULT_DELAY
    }

    /// `rd = rs ^ rt`
    pub fn xor(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rs = psx.cpu.regs.read(instr.rs());
        let rt = psx.cpu.regs.read(instr.rt());
        psx.cpu.regs.write(instr.rd(), rs ^ rt);
        self.cancel_load(instr.rd());

        DEFAULT_DELAY
    }

    /// `rt = rs ^ imm16`
    pub fn xori(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rs = psx.cpu.regs.read(instr.rs());
        let result = rs ^ u32::from(instr.imm16());
        psx.cpu.regs.write(instr.rt(), result);
        self.cancel_load(instr.rt());

        DEFAULT_DELAY
    }

    pub fn mult(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rs = i64::from(psx.cpu.regs.read(instr.rs()) as i32);
        let rt = i64::from(psx.cpu.regs.read(instr.rt()) as i32);
        let result = zerocopy::byteorder::little_endian::I64::new(rs.wrapping_mul(rt));
        let [low, high]: [zerocopy::byteorder::little_endian::U32; 2] =
            zerocopy::transmute!(result);

        psx.cpu.regs.write_lo(low.get());
        psx.cpu.regs.write_hi(high.get());

        DEFAULT_DELAY
    }

    /// `rd = rs - rt`
    pub fn sub(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rs = psx.cpu.regs.read(instr.rs()) as i32;
        let rt = psx.cpu.regs.read(instr.rt()) as i32;

        let result = rs.checked_sub(rt);
        if let Some(value) = result {
            psx.cpu.regs.write(instr.rd(), value as u32);
            self.cancel_load(instr.rd());
        } else {
            self.trigger_exception(psx, Exception::ArithmeticOverflow);
        }

        DEFAULT_DELAY
    }
}
