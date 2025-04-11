use bitos::{bitos, integer::u6};

/// The opcode of an [`Instruction`].
#[bitos(6)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    /// Perpesctive transformation single
    RTPS = 0x01,
    /// Normal clipping
    NCLIP = 0x06,
    /// Cross product of two vectors
    OP = 0x0C,
    /// Depth cueing
    DPCS = 0x10,
    /// Interpolation of vector and far color
    INTPL = 0x11,
    /// Multiply vector by matrix and add offset
    MVMVA = 0x12,
    /// Normal color depth cue single vector
    NCDS = 0x13,
    /// Average three Z values
    AVSZ3 = 0x2D,
    /// Average four Z values
    AVSZ4 = 0x2E,
    /// Perspective transformation triple
    RTPT = 0x30,
}

#[bitos(2)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MulMatrix {
    Rotation,
    Light,
    Color,
    Reserved,
}

#[bitos(2)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MulVector {
    Vector0,
    Vector1,
    Vector2,
    IR,
}

#[bitos(2)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffVector {
    Translation,
    BackgroundColor,
    FarColor,
    None,
}

/// A GTE instruction.
#[bitos(32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Instruction {
    /// The operation executed by this instruction.
    #[bits(0..6)]
    pub op: Option<Opcode>,

    /// The operation executed by this instruction.
    #[bits(0..6)]
    pub op_raw: u6,

    #[bits(10)]
    pub no_neg: bool,

    #[bits(13..15)]
    pub offset_vector: OffVector,

    #[bits(15..17)]
    pub multiply_vector: MulVector,

    #[bits(17..19)]
    pub multiply_matrix: MulMatrix,

    #[bits(19)]
    pub shift: bool,
}
