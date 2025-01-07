use super::{DEFAULT_CYCLE_COUNT, Interpreter};
use crate::cpu::{cop0::Exception, instr::Instruction};

impl Interpreter<'_> {
    /// `rt = (imm16 << 16)`
    pub fn lui(&mut self, instr: Instruction) -> u64 {
        let result = u32::from(instr.imm16()) << 16;
        self.psx.cpu.regs.write(instr.rt(), result);

        DEFAULT_CYCLE_COUNT
    }

    /// `rt = rs | imm16`
    pub fn ori(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let result = rs | u32::from(instr.imm16());
        self.psx.cpu.regs.write(instr.rt(), result);

        DEFAULT_CYCLE_COUNT
    }

    /// `rd = rt << imm5`
    pub fn sll(&mut self, instr: Instruction) -> u64 {
        let rt = self.psx.cpu.regs.read(instr.rt());
        let result = rt.unbounded_shl(u32::from(instr.imm5().value()));
        self.psx.cpu.regs.write(instr.rd(), result);

        DEFAULT_CYCLE_COUNT
    }

    /// `rt = rs + imm16`
    pub fn addiu(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let result = rs.wrapping_add_signed(i32::from(instr.signed_imm16()));
        self.psx.cpu.regs.write(instr.rt(), result);

        DEFAULT_CYCLE_COUNT
    }

    /// `rd = rs | rt`
    pub fn or(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let rt = self.psx.cpu.regs.read(instr.rt());
        self.psx.cpu.regs.write(instr.rd(), rs | rt);

        DEFAULT_CYCLE_COUNT
    }

    /// `rt = rs + signed_imm16`
    pub fn addi(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs()) as i32;
        let result = rs.checked_add(i32::from(instr.signed_imm16()));

        if let Some(value) = result {
            self.psx.cpu.regs.write(instr.rt(), value as u32);
        } else {
            self.trigger_exception(Exception::ArithmeticOverflow);
        }

        DEFAULT_CYCLE_COUNT
    }

    /// `if rs < rt { rd = 1 } else { rd = 0 }`
    pub fn sltu(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let rt = self.psx.cpu.regs.read(instr.rt());
        self.psx.cpu.regs.write(instr.rd(), u32::from(rs < rt));

        DEFAULT_CYCLE_COUNT
    }

    /// `rd = rs + rt`
    pub fn addu(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let rt = self.psx.cpu.regs.read(instr.rt());
        self.psx.cpu.regs.write(instr.rd(), rs.wrapping_add(rt));

        DEFAULT_CYCLE_COUNT
    }

    /// `rt = rs & imm16`
    pub fn andi(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let result = rs & u32::from(instr.imm16());
        self.psx.cpu.regs.write(instr.rt(), result);

        DEFAULT_CYCLE_COUNT
    }

    /// `rd = rt >> imm5`
    pub fn srl(&mut self, instr: Instruction) -> u64 {
        let rt = self.psx.cpu.regs.read(instr.rt());
        let result = rt.unbounded_shr(u32::from(instr.imm5().value()));
        self.psx.cpu.regs.write(instr.rd(), result);

        DEFAULT_CYCLE_COUNT
    }

    /// `rd = rs & rt`
    pub fn and(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let rt = self.psx.cpu.regs.read(instr.rt());
        self.psx.cpu.regs.write(instr.rd(), rs & rt);

        DEFAULT_CYCLE_COUNT
    }

    /// `rd = rs + rt`
    pub fn add(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs()) as i32;
        let rt = self.psx.cpu.regs.read(instr.rt()) as i32;

        let result = rs.checked_add(rt);
        if let Some(value) = result {
            self.psx.cpu.regs.write(instr.rd(), value as u32);
        } else {
            self.trigger_exception(Exception::ArithmeticOverflow);
        }

        DEFAULT_CYCLE_COUNT
    }

    /// `if rs < signed_imm16 { rt = 1 } else { rt = 0 }`
    pub fn slti(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs()) as i32;
        let result = rs < i32::from(instr.signed_imm16());
        self.psx.cpu.regs.write(instr.rt(), u32::from(result));

        DEFAULT_CYCLE_COUNT
    }

    /// `rd = rs - rt`
    pub fn subu(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let rt = self.psx.cpu.regs.read(instr.rt());
        self.psx.cpu.regs.write(instr.rd(), rs.wrapping_sub(rt));

        DEFAULT_CYCLE_COUNT
    }

    /// `rd = rt (signed)>> imm5`
    pub fn sra(&mut self, instr: Instruction) -> u64 {
        let rt = self.psx.cpu.regs.read(instr.rt()) as i32;
        let result = rt.unbounded_shr(u32::from(instr.imm5().value()));
        self.psx.cpu.regs.write(instr.rd(), result as u32);

        DEFAULT_CYCLE_COUNT
    }

    /// `LO = rs (signed)/ rt; HI = rs (signed)% rt`
    pub fn div(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs()) as i32;
        let rt = self.psx.cpu.regs.read(instr.rt()) as i32;
        let (div, rem) = match (rs, rt) {
            (0.., 0) => (-1, rs),
            (..0, 0) => (1, rs),
            (i32::MIN, -1) => (i32::MIN, 0),
            (rs, rt) => (
                rs.checked_div(rt).unwrap_or_default(),
                rs.checked_rem(rt).unwrap_or_default(),
            ),
        };

        self.psx.cpu.regs.lo = div as u32;
        self.psx.cpu.regs.hi = rem as u32;

        DEFAULT_CYCLE_COUNT
    }

    /// `if rs < signed_imm16 { rt = 1 } else { rt = 0 }`
    pub fn sltiu(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let result = rs < (i32::from(instr.signed_imm16()) as u32);
        self.psx.cpu.regs.write(instr.rt(), u32::from(result));

        DEFAULT_CYCLE_COUNT
    }

    /// `if rs (signed)< rt { rd = 1 } else { rd = 0 }`
    pub fn slt(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs()) as i32;
        let rt = self.psx.cpu.regs.read(instr.rt()) as i32;
        self.psx.cpu.regs.write(instr.rd(), u32::from(rs < rt));

        DEFAULT_CYCLE_COUNT
    }

    /// `LO = rs / rt; HI = rs % rt`
    pub fn divu(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let rt = self.psx.cpu.regs.read(instr.rt());
        let (div, rem) = (
            rs.checked_div(rt).unwrap_or(!0),
            rs.checked_rem(rt).unwrap_or(rs),
        );

        self.psx.cpu.regs.lo = div;
        self.psx.cpu.regs.hi = rem;

        DEFAULT_CYCLE_COUNT
    }

    /// `rd = rt << (rs & 0x1F)`
    pub fn sllv(&mut self, instr: Instruction) -> u64 {
        let rt = self.psx.cpu.regs.read(instr.rt());
        let rs = self.psx.cpu.regs.read(instr.rs());
        let result = rt.unbounded_shl(rs & 0x1F);
        self.psx.cpu.regs.write(instr.rd(), result);

        DEFAULT_CYCLE_COUNT
    }

    /// `rd = !(rs | rt)`
    pub fn nor(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let rt = self.psx.cpu.regs.read(instr.rt());
        self.psx.cpu.regs.write(instr.rd(), !(rs | rt));

        DEFAULT_CYCLE_COUNT
    }

    /// `rd = rt (signed)>> (rs & 0x1F)`
    pub fn srav(&mut self, instr: Instruction) -> u64 {
        let rt = self.psx.cpu.regs.read(instr.rt()) as i32;
        let rs = self.psx.cpu.regs.read(instr.rs());
        let result = rt.unbounded_shr(rs & 0x1F);
        self.psx.cpu.regs.write(instr.rd(), result as u32);

        DEFAULT_CYCLE_COUNT
    }

    /// `rd = rt >> (rs & 0x1F)`
    pub fn srlv(&mut self, instr: Instruction) -> u64 {
        let rt = self.psx.cpu.regs.read(instr.rt());
        let rs = self.psx.cpu.regs.read(instr.rs());
        let result = rt.unbounded_shr(rs & 0x1F);
        self.psx.cpu.regs.write(instr.rd(), result);

        DEFAULT_CYCLE_COUNT
    }

    pub fn multu(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs()) as u64;
        let rt = self.psx.cpu.regs.read(instr.rt()) as u64;
        let result = zerocopy::byteorder::little_endian::U64::new(rs * rt);
        let [low, high]: [zerocopy::byteorder::little_endian::U32; 2] =
            zerocopy::transmute!(result);

        self.psx.cpu.regs.lo = low.get();
        self.psx.cpu.regs.hi = high.get();

        DEFAULT_CYCLE_COUNT
    }

    /// `rd = rs ^ rt`
    pub fn xor(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let rt = self.psx.cpu.regs.read(instr.rt());
        self.psx.cpu.regs.write(instr.rd(), rs ^ rt);

        DEFAULT_CYCLE_COUNT
    }

    /// `rt = rs ^ imm16`
    pub fn xori(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs());
        let result = rs ^ u32::from(instr.imm16());
        self.psx.cpu.regs.write(instr.rt(), result);

        DEFAULT_CYCLE_COUNT
    }

    pub fn mult(&mut self, instr: Instruction) -> u64 {
        let rs = i64::from(self.psx.cpu.regs.read(instr.rs()) as i32);
        let rt = i64::from(self.psx.cpu.regs.read(instr.rt()) as i32);
        let result = zerocopy::byteorder::little_endian::I64::new(rs.wrapping_mul(rt));
        let [low, high]: [zerocopy::byteorder::little_endian::U32; 2] =
            zerocopy::transmute!(result);

        self.psx.cpu.regs.lo = low.get();
        self.psx.cpu.regs.hi = high.get();

        DEFAULT_CYCLE_COUNT
    }

    /// `rd = rs - rt`
    pub fn sub(&mut self, instr: Instruction) -> u64 {
        let rs = self.psx.cpu.regs.read(instr.rs()) as i32;
        let rt = self.psx.cpu.regs.read(instr.rt()) as i32;

        let result = rs.checked_sub(rt);
        if let Some(value) = result {
            self.psx.cpu.regs.write(instr.rd(), value as u32);
        } else {
            self.trigger_exception(Exception::ArithmeticOverflow);
        }

        DEFAULT_CYCLE_COUNT
    }
}
