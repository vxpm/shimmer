//! [`Instruction`], which represents a single MIPS I instruction, and related items.

use super::{COP, Reg};
use bitos::{
    bitos,
    integer::{u4, u5, u20, u25, u26},
};
use strum::IntoStaticStr;

/// The opcode of an [`Instruction`].
#[bitos(6)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoStaticStr)]
pub enum Opcode {
    SPECIAL = 0x00,
    BZ = 0x01,
    JMP = 0x02,
    JAL = 0x03,
    BEQ = 0x04,
    BNE = 0x05,
    BLEZ = 0x06,
    BGTZ = 0x07,
    ADDI = 0x08,
    ADDIU = 0x09,
    SLTI = 0x0A,
    SLTIU = 0x0B,
    ANDI = 0x0C,
    ORI = 0x0D,
    XORI = 0x0E,
    LUI = 0x0F,
    COP0 = 0x10,
    COP1 = 0x11,
    COP2 = 0x12,
    COP3 = 0x13,
    LB = 0x20,
    LH = 0x21,
    LWL = 0x22,
    LW = 0x23,
    LBU = 0x24,
    LHU = 0x25,
    LWR = 0x26,
    SB = 0x28,
    SH = 0x29,
    SWL = 0x2A,
    SW = 0x2B,
    SWR = 0x2E,
    LWC0 = 0x30,
    LWC1 = 0x31,
    LWC2 = 0x32,
    LWC3 = 0x33,
    SWC0 = 0x38,
    SWC1 = 0x39,
    SWC2 = 0x3A,
    SWC3 = 0x3B,
}

#[bitos(5)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoStaticStr)]
pub enum BZKind {
    BLTZ,
    BGEZ,
    BLTZAL,
    BGEZAL,
}

/// The special opcode of an [`Instruction`] whose primary opcode is [`Opcode::SPECIAL`].
#[bitos(6)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoStaticStr)]
pub enum SpecialOpcode {
    SLL = 0x00,
    SRL = 0x02,
    SRA = 0x03,
    SLLV = 0x04,
    SRLV = 0x06,
    SRAV = 0x07,
    JR = 0x08,
    JALR = 0x09,
    SYSCALL = 0x0C,
    BREAK = 0x0D,
    MFHI = 0x10,
    MTHI = 0x11,
    MFLO = 0x12,
    MTLO = 0x13,
    MULT = 0x18,
    MULTU = 0x19,
    DIV = 0x1A,
    DIVU = 0x1B,
    ADD = 0x20,
    ADDU = 0x21,
    SUB = 0x22,
    SUBU = 0x23,
    AND = 0x24,
    OR = 0x25,
    XOR = 0x26,
    NOR = 0x27,
    SLT = 0x2A,
    SLTU = 0x2B,
}

#[bitos(4)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoStaticStr)]
pub enum CopOpcode {
    MFC = 0x00,
    CFC = 0x02,
    MTC = 0x04,
    CTC = 0x06,
    BRANCH = 0x08,
}

#[bitos(6)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoStaticStr)]
pub enum SpecialCop0Opcode {
    RFE = 0x10,
}

/// A MIPS I instruction.
#[bitos(32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Instruction {
    /// The operation executed by this instruction.
    #[bits(26..32)]
    pub op: Option<Opcode>,

    #[bits(17..21)]
    pub bz_link: u4,

    #[bits(16)]
    pub bz_ge: bool,

    /// The operation executed by this instruction if it's primary opcode is [`Opcode::SPECIAL`].
    #[bits(0..6)]
    pub special_op: Option<SpecialOpcode>,

    #[bits(26..28)]
    pub cop: COP,

    #[bits(25)]
    pub cop_cmd: bool,

    #[bits(21..25)]
    pub cop_op: Option<CopOpcode>,

    #[bits(0..6)]
    pub cop0_special_op: Option<SpecialCop0Opcode>,

    /// The destination register of this instruction.
    #[bits(11..16)]
    pub rd: Reg,

    /// The destination register of this instruction.
    #[bits(11..16)]
    pub cop0_rd: crate::cpu::cop0::Reg,

    /// The destination register of this instruction.
    #[bits(11..16)]
    pub gte_data_rd: crate::gte::DataReg,

    /// The destination register of this instruction.
    #[bits(11..16)]
    pub gte_control_rd: crate::gte::ControlReg,

    /// The target register of this instruction.
    #[bits(16..21)]
    pub rt: Reg,

    /// The target register of this instruction.
    #[bits(11..16)]
    pub cop0_rt: crate::cpu::cop0::Reg,

    /// The target register of this instruction.
    #[bits(16..21)]
    pub gte_data_rt: crate::gte::DataReg,

    /// The source register of this instruction.
    #[bits(21..26)]
    pub rs: Reg,

    #[bits(6..11)]
    pub imm5: u5,

    /// The unsigned 16 bit immediate value of this instruction.
    #[bits(0..16)]
    pub imm16: u16,

    /// The signed 16 bit immediate value of this instruction.
    #[bits(0..16)]
    pub signed_imm16: i16,

    /// The 20 bit immediate value of this instruction.
    #[bits(6..26)]
    pub imm20: u20,

    /// The 25 bit immediate value of this instruction. Used only by COP2.
    #[bits(0..25)]
    pub imm25: u25,

    /// The 26 bit immediate value of this instruction.
    #[bits(0..26)]
    pub imm26: u26,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegSource {
    CPU,
    COP0,
    COP1,
    COP2,
    COP3,
}

#[derive(Debug, Clone, Copy)]
pub enum ImmKind {
    U5,
    U16,
    I16,
    U20,
    U26,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Args {
    pub rd: Option<RegSource>,
    pub rs: Option<RegSource>,
    pub rt: Option<RegSource>,
    pub imm: Option<ImmKind>,
}

macro_rules! args {
    (@$current:expr; $arg:ident: $src:ident; $($remainder:tt)*) => {
        {
            let mut value = $current;
            value.$arg = Some(RegSource::$src);
            args!(@value; $($remainder)*)
        }
    };
    (@$current:expr; $arg:ident: $src:expr; $($remainder:tt)*) => {
        {
            let mut value = $current;
            value.$arg = Some($src);
            args!(@value; $($remainder)*)
        }
    };
    (@$current:expr; $imm:ident; $($remainder:tt)*) => {
        {
            let mut value = $current;
            value.imm = Some(ImmKind::$imm);
            args!(@value; $($remainder)*)
        }
    };
    (@$current:expr;) => {
        $current
    };
    ($($tokens:tt)*) => {
        {
            let value = Args { rs: None, rd: None, rt: None, imm: None };
            args!(@value; $($tokens)*)
        }
    }
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Some(mnemonic) = self.mnemonic() else {
            return write!(f, "ILLEGAL");
        };
        write!(f, "{mnemonic}")?;

        let args = self.args().unwrap();
        let prefix = |src| match src {
            RegSource::CPU => "",
            RegSource::COP0 => "COP0_",
            RegSource::COP1 => "COP1_",
            RegSource::COP2 => "COP2_",
            RegSource::COP3 => "COP3_",
        };

        let mut is_first = true;
        let mut write_comma = |f: &mut std::fmt::Formatter| {
            if is_first {
                is_first = false;
                Ok(())
            } else {
                write!(f, ",")
            }
        };

        if let Some(src) = args.rd {
            let prefix = prefix(src);
            let rd = self.rd();

            write_comma(f)?;
            write!(f, " {prefix}{}", rd.alt_name())?;
        }

        if let Some(src) = args.rt {
            let prefix = prefix(src);
            let rt = self.rt();

            write_comma(f)?;
            write!(f, " {prefix}{}", rt.alt_name())?;
        }

        if let Some(src) = args.rs {
            let prefix = prefix(src);
            let rs = self.rs();

            write_comma(f)?;
            write!(f, " {prefix}{}", rs.alt_name())?;
        }

        if let Some(imm) = args.imm {
            write_comma(f)?;
            match imm {
                ImmKind::U5 => write!(f, " 0x{:02X}", self.imm5())?,
                ImmKind::U16 => write!(f, " 0x{:04X}", self.imm16())?,
                ImmKind::I16 => write!(f, " 0x{:04X}", self.signed_imm16())?,
                ImmKind::U20 => write!(f, " 0x{:05X}", self.imm20())?,
                ImmKind::U26 => write!(f, " 0x{:06X}", self.imm26())?,
            }
        }

        Ok(())
    }
}

impl Instruction {
    pub const NOP: Self = Instruction(0x0000_0000);

    pub fn args(&self) -> Option<Args> {
        Some(match self.op()? {
            Opcode::SPECIAL => match self.special_op()? {
                SpecialOpcode::SLL => args!(rd: CPU; rt: CPU; U5;),
                SpecialOpcode::SRL => args!(rd: CPU; rt: CPU; U5;),
                SpecialOpcode::SRA => args!(rd: CPU; rt: CPU; U5;),
                SpecialOpcode::SLLV => args!(rd: CPU; rs: CPU; rt: CPU;),
                SpecialOpcode::SRLV => args!(rd: CPU; rs: CPU; rt: CPU;),
                SpecialOpcode::SRAV => args!(rd: CPU; rs: CPU; rt: CPU;),
                SpecialOpcode::JR => args!(rs: CPU;),
                SpecialOpcode::JALR => args!(rd: CPU; rs: CPU;),
                SpecialOpcode::SYSCALL => args!(U20;),
                SpecialOpcode::BREAK => args!(U20;),
                SpecialOpcode::MFHI => args!(rd: CPU;),
                SpecialOpcode::MTHI => args!(rs: CPU;),
                SpecialOpcode::MFLO => args!(rd: CPU;),
                SpecialOpcode::MTLO => args!(rs: CPU;),
                SpecialOpcode::MULT => args!(rs: CPU; rt: CPU;),
                SpecialOpcode::MULTU => args!(rs: CPU; rt: CPU;),
                SpecialOpcode::DIV => args!(rs: CPU; rt: CPU;),
                SpecialOpcode::DIVU => args!(rs: CPU; rt: CPU;),
                SpecialOpcode::ADD => args!(rd: CPU; rs: CPU; rt: CPU;),
                SpecialOpcode::ADDU => args!(rd: CPU; rs: CPU; rt: CPU;),
                SpecialOpcode::SUB => args!(rd: CPU; rs: CPU; rt: CPU;),
                SpecialOpcode::SUBU => args!(rd: CPU; rs: CPU; rt: CPU;),
                SpecialOpcode::AND => args!(rd: CPU; rs: CPU; rt: CPU;),
                SpecialOpcode::OR => args!(rd: CPU; rs: CPU; rt: CPU;),
                SpecialOpcode::XOR => args!(rd: CPU; rs: CPU; rt: CPU;),
                SpecialOpcode::NOR => args!(rd: CPU; rs: CPU; rt: CPU;),
                SpecialOpcode::SLT => args!(rd: CPU; rs: CPU; rt: CPU;),
                SpecialOpcode::SLTU => args!(rd: CPU; rs: CPU; rt: CPU;),
            },
            Opcode::BZ => args!(rs: CPU; I16;),
            Opcode::JMP => args!(U26;),
            Opcode::JAL => args!(U26;),
            Opcode::BEQ => args!(rs: CPU; rt: CPU; U16;),
            Opcode::BNE => args!(rs: CPU; rt: CPU; U16;),
            Opcode::BLEZ => args!(rs: CPU; U16;),
            Opcode::BGTZ => args!(rs: CPU; U16;),
            Opcode::ADDI => args!(rs: CPU; rt: CPU; I16;),
            Opcode::ADDIU => args!(rs: CPU; rt: CPU; U16;),
            Opcode::SLTI => args!(rs: CPU; rt: CPU; I16;),
            Opcode::SLTIU => args!(rs: CPU; rt: CPU; U16;),
            Opcode::ANDI => args!(rs: CPU; rt: CPU; U16;),
            Opcode::ORI => args!(rs: CPU; rt: CPU; U16;),
            Opcode::XORI => args!(rs: CPU; rt: CPU; U16;),
            Opcode::LUI => args!(rt: CPU; U16;),
            Opcode::COP0 => match self.cop_op()? {
                CopOpcode::MFC => args!(rd: COP0; rt: CPU;),
                CopOpcode::CFC => args!(rd: COP0; rt: CPU;),
                CopOpcode::MTC => args!(rd: COP0; rt: CPU;),
                CopOpcode::CTC => args!(rd: COP0; rt: CPU;),
                CopOpcode::BRANCH => args!(),
            },
            Opcode::COP1 => match self.cop_op()? {
                CopOpcode::MFC => args!(rd: COP1; rt: CPU;),
                CopOpcode::CFC => args!(rd: COP1; rt: CPU;),
                CopOpcode::MTC => args!(rd: COP1; rt: CPU;),
                CopOpcode::CTC => args!(rd: COP1; rt: CPU;),
                CopOpcode::BRANCH => args!(),
            },
            Opcode::COP2 => match self.cop_op()? {
                CopOpcode::MFC => args!(rd: COP2; rt: CPU;),
                CopOpcode::CFC => args!(rd: COP2; rt: CPU;),
                CopOpcode::MTC => args!(rd: COP2; rt: CPU;),
                CopOpcode::CTC => args!(rd: COP2; rt: CPU;),
                CopOpcode::BRANCH => args!(),
            },
            Opcode::COP3 => match self.cop_op()? {
                CopOpcode::MFC => args!(rd: COP3; rt: CPU;),
                CopOpcode::CFC => args!(rd: COP3; rt: CPU;),
                CopOpcode::MTC => args!(rd: COP3; rt: CPU;),
                CopOpcode::CTC => args!(rd: COP3; rt: CPU;),
                CopOpcode::BRANCH => args!(),
            },
            Opcode::LB => args!(rs: CPU; rt: CPU; U16;),
            Opcode::LH => args!(rs: CPU; rt: CPU; U16;),
            Opcode::LWL => args!(rs: CPU; rt: CPU; U16;),
            Opcode::LW => args!(rs: CPU; rt: CPU; U16;),
            Opcode::LBU => args!(rs: CPU; rt: CPU; U16;),
            Opcode::LHU => args!(rs: CPU; rt: CPU; U16;),
            Opcode::LWR => args!(rs: CPU; rt: CPU; U16;),
            Opcode::SB => args!(rs: CPU; rt: CPU; U16;),
            Opcode::SH => args!(rs: CPU; rt: CPU; U16;),
            Opcode::SWL => args!(rs: CPU; rt: CPU; U16;),
            Opcode::SW => args!(rs: CPU; rt: CPU; U16;),
            Opcode::SWR => args!(rs: CPU; rt: CPU; U16;),
            Opcode::LWC0 => args!(rs: CPU; rt: COP0; U16;),
            Opcode::LWC1 => args!(rs: CPU; rt: COP0; U16;),
            Opcode::LWC2 => args!(rs: CPU; rt: COP2; U16;),
            Opcode::LWC3 => args!(rs: CPU; rt: COP0; U16;),
            Opcode::SWC0 => args!(rs: CPU; rt: COP0; U16;),
            Opcode::SWC1 => args!(rs: CPU; rt: COP1; U16;),
            Opcode::SWC2 => args!(rs: CPU; rt: COP2; U16;),
            Opcode::SWC3 => args!(rs: CPU; rt: COP3; U16;),
        })
    }

    pub fn bz_kind(&self) -> BZKind {
        match (self.bz_ge(), self.bz_link().value() == 0b1000) {
            (true, true) => BZKind::BGEZAL,
            (true, false) => BZKind::BGEZ,
            (false, true) => BZKind::BLTZAL,
            (false, false) => BZKind::BLTZ,
        }
    }

    /// Returns the mnemonic of this instruction.
    pub fn mnemonic(&self) -> Option<String> {
        if self.op() == Some(Opcode::SPECIAL) {
            self.special_op()
                .map(|s| <&'static str>::from(s).to_owned())
        } else if self.op() == Some(Opcode::BZ) {
            Some(<&'static str>::from(self.bz_kind()).to_owned())
        } else if matches!(self.op(), Some(Opcode::COP0 | Opcode::COP2)) {
            let cop: &'static str = self.cop().into();
            if self.cop_cmd() {
                // TODO: fix this
                self.cop0_special_op()
                    .map(|op| format!("{cop}_{}", <&'static str>::from(op)))
            } else {
                self.cop_op()
                    .map(|op| format!("{cop}_{}", <&'static str>::from(op)))
            }
        } else {
            self.op().map(|s| <&'static str>::from(s).to_owned())
        }
    }

    pub fn is_illegal(&self) -> bool {
        #[expect(clippy::match_like_matches_macro, reason = "more readable as a match")]
        match (self.op(), self.special_op(), self.cop_op()) {
            (None, _, _) => true,
            (Some(Opcode::SPECIAL), None, _) => true,
            (Some(Opcode::COP0 | Opcode::COP2), _, None) => true,
            _ => false,
        }
    }
}

impl Default for Instruction {
    fn default() -> Self {
        Self::NOP
    }
}
