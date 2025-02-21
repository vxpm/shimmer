//! Items related to textures.

use bitos::{
    bitos,
    integer::{u1, u4, u5, u6, u9},
};

/// The bit depth of a texture.
#[bitos(2)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Depth {
    Nibble = 0,
    Byte = 1,
    Full = 2,
    Reserved = 3,
}

/// The blending mode of a texture.
#[bitos(2)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendingMode {
    /// Final Color = Old / 2 + New / 2
    Half = 0,
    /// Final Color = Old + New
    Add = 1,
    /// Final Color = Old - New
    Sub = 2,
    /// Final Color = Old + New / 4
    Quarter = 3,
}

/// A texture page.
#[bitos(9)]
#[derive(Debug, Clone, Copy, Default)]
pub struct TexPage {
    #[bits(0..4)]
    pub x_base: u4,
    #[bits(4)]
    pub y_base: u1,
    #[bits(5..7)]
    pub blending_mode: BlendingMode,
    #[bits(7..9)]
    pub depth: Depth,
}

/// A texture window.
#[bitos(20)]
#[derive(Debug, Clone, Copy, Default)]
pub struct TexWindow {
    #[bits(0..5)]
    pub mask_x: u5,
    #[bits(5..10)]
    pub mask_y: u5,
    #[bits(10..15)]
    pub offset_x: u5,
    #[bits(15..20)]
    pub offset_y: u5,
}

/// Color LookUp table coordinates.
#[bitos(16)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Clut {
    #[bits(0..6)]
    pub x_by_16: u6,
    #[bits(6..15)]
    pub y: u9,
}
