//! Rendering commands.

use crate::gpu::texture::{Clut, TexPage};
use bitos::{bitos, integer::i11};

/// A framebuffer transfer coordinate packet.
#[bitos(32)]
#[derive(Debug, Clone)]
pub struct CoordPacket {
    #[bits(0..16)]
    pub x: u16,
    #[bits(16..32)]
    pub y: u16,
}

/// A framebuffer transfer dimensions packet.
#[bitos(32)]
#[derive(Debug, Clone)]
pub struct SizePacket {
    #[bits(0..16)]
    pub width: u16,
    #[bits(16..32)]
    pub height: u16,
}

/// The texture mode of a rendering command.
#[bitos(1)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureMode {
    Modulated = 0,
    Raw = 1,
}

/// The blending mode of a rendering command.
#[bitos(1)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendingMode {
    Opaque = 0,
    SemiTransparent = 1,
}

/// The shading mode of a rendering command.
#[bitos(1)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadingMode {
    Flat = 0,
    Gouraud = 1,
}

/// A vertex color packet.
#[bitos(32)]
#[derive(Debug, Clone, Copy, Default)]
pub struct VertexColorPacket {
    #[bits(0..8)]
    pub r: u8,
    #[bits(8..16)]
    pub g: u8,
    #[bits(16..24)]
    pub b: u8,
}

/// A vertex position packet.
#[bitos(32)]
#[derive(Debug, Clone, Copy, Default)]
pub struct VertexPositionPacket {
    #[bits(0..11)]
    pub x: i11,
    #[bits(16..27)]
    pub y: i11,
}

impl VertexPositionPacket {
    pub fn apply_offset(&mut self, x: i11, y: i11) {
        self.set_x(i11::new(self.x().value() + x.value()));
        self.set_y(i11::new(self.y().value() + y.value()));
    }
}

/// A vertex UV packet.
#[bitos(32)]
#[derive(Debug, Clone, Default)]
pub struct VertexUVPacket {
    #[bits(0..8)]
    pub u: u8,
    #[bits(8..16)]
    pub v: u8,

    #[bits(16..32)]
    pub clut: Clut,
    #[bits(16..25)]
    pub texpage: TexPage,
    #[bits(26)]
    pub texture_disable: bool,
}

/// The Polygon mode of a [`PolygonCmd`].
#[bitos(1)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolygonMode {
    Triangle = 0,
    Rectangle = 1,
}

impl PolygonMode {
    pub fn vertices(&self) -> usize {
        match self {
            PolygonMode::Triangle => 3,
            PolygonMode::Rectangle => 4,
        }
    }
}

/// A polygon rendering command.
#[bitos(32)]
#[derive(Debug)]
pub struct PolygonCmd {
    #[bits(0..8)]
    pub r: u8,
    #[bits(8..16)]
    pub g: u8,
    #[bits(16..24)]
    pub b: u8,
    #[bits(24)]
    pub texture_mode: TextureMode,
    #[bits(25)]
    pub blending_mode: BlendingMode,
    #[bits(26)]
    pub textured: bool,
    #[bits(27)]
    pub polygon_mode: PolygonMode,
    #[bits(28)]
    pub shading_mode: ShadingMode,
}

/// The line mode of a [`LineCmd`].
#[bitos(1)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineMode {
    Single = 0,
    Poly = 1,
}

/// A line rendering command.
#[bitos(32)]
#[derive(Debug, Clone)]
pub struct LineCmd {
    #[bits(0..8)]
    pub color_r: u8,
    #[bits(8..16)]
    pub color_g: u8,
    #[bits(16..24)]
    pub color_b: u8,
    #[bits(25)]
    pub transparency_mode: BlendingMode,
    #[bits(27)]
    pub line_mode: LineMode,
    #[bits(28)]
    pub shading_mode: ShadingMode,
}

/// The rectangle mode of a [`RectangleCmd`].
#[bitos(2)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RectangleMode {
    Variable = 0,
    SinglePixel = 1,
    Sprite8 = 2,
    Sprite16 = 3,
}

/// A rectangle rendering command.
#[bitos(32)]
#[derive(Debug)]
pub struct RectangleCmd {
    #[bits(0..8)]
    pub r: u8,
    #[bits(8..16)]
    pub g: u8,
    #[bits(16..24)]
    pub b: u8,
    #[bits(24)]
    pub texture_mode: TextureMode,
    #[bits(25)]
    pub blending_mode: BlendingMode,
    #[bits(26)]
    pub textured: bool,
    #[bits(27..29)]
    pub rectangle_mode: RectangleMode,
}
