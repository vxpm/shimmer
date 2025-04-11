//! Items related to the CPU of the PSX, the R3000.

pub mod cop0;
pub mod instr;

use crate::mem;
use bitos::bitos;
use strum::{EnumMessage, IntoStaticStr, VariantArray};

/// The frequency of the CPU, in Hz.
pub const FREQUENCY: u32 = 33_870_000;

/// A CPU coprocessor kind.
#[bitos(2)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoStaticStr)]
pub enum COP {
    /// System Coprocessor
    COP0,
    /// Floating Point Unit, absent in the PSX
    COP1,
    /// GTE
    COP2,
    /// Absent in the PSX
    COP3,
}

impl COP {
    pub fn opcode(&self) -> instr::Opcode {
        match self {
            COP::COP0 => instr::Opcode::COP0,
            COP::COP1 => instr::Opcode::COP1,
            COP::COP2 => instr::Opcode::COP2,
            COP::COP3 => instr::Opcode::COP3,
        }
    }
}

/// A general purpose register of the CPU.
#[bitos(5)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, VariantArray, EnumMessage)]
pub enum Reg {
    /// `R0`, the only register with a constant value: it always evaluates to zero.
    R0,
    /// `R1`, also called `AT`, is the Assembler Temporary register. It is reserved for assemblers
    /// and is usually left untouched otherwise.
    R1,
    /// `R2`, also called `V0`, is a register used as a subroutine return value.
    R2,
    /// `R3`, also called `V1`, is a register used as a subroutine return value.
    R3,
    /// `R4`, also called `A0`, is a register used as a subroutine argument.
    R4,
    /// `R5`, also called `A1`, is a register used as a subroutine argument.
    R5,
    /// `R6`, also called `A2`, is a register used as a subroutine argument.
    R6,
    /// `R7`, also called `A3`, is a register used as a subroutine argument.
    R7,
    /// `R8`, also called `T0`, is a register used as a subroutine temporary. It may be used by
    /// subroutines without preserving it for the calling routine.
    R8,
    /// `R9`, also called `T1`, is a register used as a subroutine temporary. It may be used by
    /// subroutines without preserving it for the calling routine.
    R9,
    /// `R10`, also called `T2`, is a register used as a subroutine temporary. It may be used by
    /// subroutines without preserving it for the calling routine.
    R10,
    /// `R11`, also called `T3`, is a register used as a subroutine temporary. It may be used by
    /// subroutines without preserving it for the calling routine.
    R11,
    /// `R12`, also called `T4`, is a register used as a subroutine temporary. It may be used by
    /// subroutines without preserving it for the calling routine.
    R12,
    /// `R13`, also called `T5`, is a register used as a subroutine temporary. It may be used by
    /// subroutines without preserving it for the calling routine.
    R13,
    /// `R14`, also called `T6`, is a register used as a subroutine temporary. It may be used by
    /// subroutines without preserving it for the calling routine.
    R14,
    /// `R15`, also called `T7`, is a register used as a subroutine temporary. It may be used by
    /// subroutines without preserving it for the calling routine.
    R15,
    /// `R16`, also called `S0`, is a register used as a subroutine register variable. It's value
    /// must be saved and restored before the subroutine exits.
    R16,
    /// `R17`, also called `S1`, is a register used as a subroutine register variable. It's value
    /// must be saved and restored before the subroutine exits.
    R17,
    /// `R18`, also called `S2`, is a register used as a subroutine register variable. It's value
    /// must be saved and restored before the subroutine exits.
    R18,
    /// `R19`, also called `S3`, is a register used as a subroutine register variable. It's value
    /// must be saved and restored before the subroutine exits.
    R19,
    /// `R20`, also called `S4`, is a register used as a subroutine register variable. It's value
    /// must be saved and restored before the subroutine exits.
    R20,
    /// `R21`, also called `S5`, is a register used as a subroutine register variable. It's value
    /// must be saved and restored before the subroutine exits.
    R21,
    /// `R22`, also called `S6`, is a register used as a subroutine register variable. It's value
    /// must be saved and restored before the subroutine exits.
    R22,
    /// `R23`, also called `S7`, is a register used as a subroutine register variable. It's value
    /// must be saved and restored before the subroutine exits.
    R23,
    /// `R24`, also called `T8`, is a register used as a subroutine temporary. It may be used by
    /// subroutines without preserving it for the calling routine.
    R24,
    /// `R25`, also called `T9`, is a register used as a subroutine temporary. It may be used by
    /// subroutines without preserving it for the calling routine.
    R25,
    /// `R26`, also called `K0`, is a register reserved for the Kernel. It may be destroyed by
    /// interrupt handlers.
    R26,
    /// `R27`, also called `K1`, is a register reserved for the Kernel. It may be destroyed by
    /// interrupt handlers.
    R27,
    /// `R28`, also called `GP`, is a register usually used by some runtime systems as a global
    /// pointer to some static data.
    R28,
    /// `R29`, also called `SP`, is a register used as a stack pointer. It points to the top of the
    /// stack.
    R29,
    /// `R30`, also called `FP`, is a register used as a frame pointer. It points to the start of
    /// the current stack frame and must be saved and restored before a subroutine exits.
    R30,
    /// `R31`, also called `RA`, is a register used as a return address for subroutines. It is
    /// modified by instructions of the `AL` family, like `JAL`.
    R31,
}

impl Reg {
    pub const ZERO: Reg = Reg::R0;
    pub const AT: Reg = Reg::R1;

    pub const V0: Reg = Reg::R2;
    pub const V1: Reg = Reg::R3;

    pub const A0: Reg = Reg::R4;
    pub const A1: Reg = Reg::R5;
    pub const A2: Reg = Reg::R6;
    pub const A3: Reg = Reg::R7;

    pub const T0: Reg = Reg::R8;
    pub const T1: Reg = Reg::R9;
    pub const T2: Reg = Reg::R10;
    pub const T3: Reg = Reg::R11;
    pub const T4: Reg = Reg::R12;
    pub const T5: Reg = Reg::R13;
    pub const T6: Reg = Reg::R14;
    pub const T7: Reg = Reg::R15;

    pub const S0: Reg = Reg::R16;
    pub const S1: Reg = Reg::R17;
    pub const S2: Reg = Reg::R18;
    pub const S3: Reg = Reg::R19;
    pub const S4: Reg = Reg::R20;
    pub const S5: Reg = Reg::R21;
    pub const S6: Reg = Reg::R22;
    pub const S7: Reg = Reg::R23;

    pub const T8: Reg = Reg::R24;
    pub const T9: Reg = Reg::R25;

    pub const K0: Reg = Reg::R26;
    pub const K1: Reg = Reg::R27;

    pub const GP: Reg = Reg::R28;
    pub const SP: Reg = Reg::R29;
    pub const FP: Reg = Reg::R30;
    pub const RA: Reg = Reg::R31;

    pub fn alt_name(&self) -> &'static str {
        match self {
            Reg::R0 => "00",
            Reg::R1 => "AT",
            Reg::R2 => "V0",
            Reg::R3 => "V1",
            Reg::R4 => "A0",
            Reg::R5 => "A1",
            Reg::R6 => "A2",
            Reg::R7 => "A3",
            Reg::R8 => "T0",
            Reg::R9 => "T1",
            Reg::R10 => "T2",
            Reg::R11 => "T3",
            Reg::R12 => "T4",
            Reg::R13 => "T5",
            Reg::R14 => "T6",
            Reg::R15 => "T7",
            Reg::R16 => "S0",
            Reg::R17 => "S1",
            Reg::R18 => "S2",
            Reg::R19 => "S3",
            Reg::R20 => "S4",
            Reg::R21 => "S5",
            Reg::R22 => "S6",
            Reg::R23 => "S7",
            Reg::R24 => "T8",
            Reg::R25 => "T9",
            Reg::R26 => "K0",
            Reg::R27 => "K1",
            Reg::R28 => "GP",
            Reg::R29 => "SP",
            Reg::R30 => "FP",
            Reg::R31 => "RA",
        }
    }

    pub fn description(&self) -> &'static str {
        self.get_documentation().unwrap()
    }
}

/// The registers of the CPU.
#[derive(Clone)]
pub struct Registers {
    gp: [u32; 32],
    hi: u32,
    lo: u32,
    pc: u32,
}

impl std::fmt::Debug for Registers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Registers")
            .field_with("gp", |f| {
                let mut map = f.debug_map();
                for i in 0..32 {
                    if self.gp[i as usize] != 0 {
                        map.entry(
                            &unsafe { std::mem::transmute::<u8, Reg>(i) },
                            &self.gp[i as usize],
                        );
                    }
                }

                map.finish_non_exhaustive()
            })
            .field("hi", &self.hi)
            .field("lo", &self.lo)
            .field("pc", &self.pc)
            .finish()
    }
}

impl Default for Registers {
    fn default() -> Self {
        Self {
            gp: Default::default(),
            hi: Default::default(),
            lo: Default::default(),
            pc: mem::Region::BIOS.start().value() + mem::Segment::KSEG1.start().value(),
        }
    }
}

impl Registers {
    #[inline(always)]
    pub fn read(&self, reg: Reg) -> u32 {
        self.gp[reg as usize]
    }

    #[inline(always)]
    pub fn write(&mut self, reg: Reg, value: u32) {
        if reg != Reg::R0 {
            self.gp[reg as usize] = value;
        }
    }

    #[inline(always)]
    pub fn read_pc(&self) -> u32 {
        self.pc
    }

    #[inline(always)]
    pub fn write_pc(&mut self, value: u32) {
        self.pc = value;
    }

    #[inline(always)]
    pub fn read_lo(&self) -> u32 {
        self.lo
    }

    #[inline(always)]
    pub fn write_lo(&mut self, value: u32) {
        self.lo = value;
    }

    #[inline(always)]
    pub fn read_hi(&self) -> u32 {
        self.hi
    }

    #[inline(always)]
    pub fn write_hi(&mut self, value: u32) {
        self.hi = value;
    }
}

/// A pending load operation, usually in the delay slot.
#[derive(Debug, Clone, Copy)]
pub struct RegLoad {
    pub reg: Reg,
    pub value: u32,
}

/// The state of the CPU.
#[derive(Debug, Clone, Default)]
pub struct Cpu {
    pub regs: Registers,
    pub cache_control: u32,
}
