use bitos::{
    bitos,
    integer::{i11, u6, u9},
};

/// The texture mode of a rendering instruction.
#[bitos(1)]
#[derive(Debug, PartialEq, Eq)]
pub enum TextureMode {
    Modulated = 0,
    Raw = 1,
}

/// The transparency mode of a rendering instruction.
#[bitos(1)]
#[derive(Debug, PartialEq, Eq)]
pub enum TransparencyMode {
    Opaque = 0,
    SemiTransparent = 1,
}

/// The shading mode of a rendering instruction.
#[bitos(1)]
#[derive(Debug, PartialEq, Eq)]
pub enum ShadingMode {
    Flat = 0,
    Gouraud = 1,
}

/// A vertex color packet.
#[bitos(32)]
#[derive(Debug)]
pub struct VertexColorPacket {
    #[bits(0..8)]
    color_r: u8,
    #[bits(8..16)]
    color_g: u8,
    #[bits(16..24)]
    color_b: u8,
}

/// A vertex position packet.
#[bitos(32)]
#[derive(Debug)]
pub struct VertexPositionPacket {
    #[bits(0..11)]
    pub x: i11,
    #[bits(16..27)]
    pub y: i11,
}

#[bitos(16)]
#[derive(Debug)]
pub struct Clut {
    #[bits(0..6)]
    x_by_16: u6,
    #[bits(6..15)]
    y: u9,
}

// TODO: finish this
#[bitos(16)]
#[derive(Debug)]
pub struct TexPage {}

/// A vertex UV packet.
#[bitos(32)]
#[derive(Debug)]
pub struct VertexUVPacket {
    #[bits(0..8)]
    u: u8,
    #[bits(8..16)]
    v: u8,
    #[bits(16..32)]
    clut: Clut,
    #[bits(16..32)]
    page: TexPage,
}

/// The Polygon mode of a [`PolygonInstr`].
#[bitos(1)]
#[derive(Debug, PartialEq, Eq)]
pub enum PolygonMode {
    Triangle = 0,
    Rectangle = 1,
}

/// A polygon rendering instruction. This instruction always requires some data packets, with the
/// amount changing depending on some of it's values.
///
/// The data required by this instruction is vertex data, and it is received in the following
/// sequence:
/// - If doing gouraud shading, a [`VertexColorPacket`].
/// - A [`VertexPositionPacket`].
/// - If doing textured polygons, a [`VertexUVPacket`].
///
/// If the `polygon_mode` of this instruction is [`PolygonMode::Triangle`], 3 vertices are
/// required. Otherwise, 4 are required. Pretty intuitive!
#[bitos(32)]
#[derive(Debug)]
pub struct PolygonInstr {
    #[bits(0..8)]
    pub color_r: u8,
    #[bits(8..16)]
    pub color_g: u8,
    #[bits(16..24)]
    pub color_b: u8,
    #[bits(24)]
    pub texture_mode: TextureMode,
    #[bits(25)]
    pub transparency_mode: TransparencyMode,
    #[bits(26)]
    pub textured: bool,
    #[bits(27)]
    pub polygon_mode: PolygonMode,
    #[bits(28)]
    pub shading_mode: ShadingMode,
}

/// The line mode of a [`LineInstr`].
#[bitos(1)]
#[derive(Debug, PartialEq, Eq)]
pub enum LineMode {
    Single = 0,
    Poly = 1,
}

/// A line rendering instruction. This instruction always requires some data packets, with the
/// amount changing depending on some of it's values.
///
/// The data required by this instruction is vertex data, and it is received in the following
/// sequence:
/// - If doing gouraud shading, a [`VertexColorPacket`].
/// - A [`VertexPositionPacket`].
///
/// If the `polyline` mode is enabled, vertexes are received forever until a packet equal to
/// 0x5000_5000 is received.
#[bitos(32)]
#[derive(Debug)]
pub struct LineInstr {
    #[bits(0..8)]
    pub color_r: u8,
    #[bits(8..16)]
    pub color_g: u8,
    #[bits(16..24)]
    pub color_b: u8,
    #[bits(25)]
    pub transparency_mode: TransparencyMode,
    #[bits(27)]
    pub line_mode: LineMode,
    #[bits(28)]
    pub shading_mode: ShadingMode,
}

/// The rectangle mode of a [`RectangleInstr`].
#[bitos(2)]
#[derive(Debug, PartialEq, Eq)]
pub enum RectangleMode {
    Variable = 0,
    SinglePixel = 1,
    Sprite8 = 2,
    Sprite16 = 3,
}

/// A rectangle rendering instruction. This instruction always requires some data packets, with the
/// amount changing depending on some of it's values.
///
/// The data required by this instruction is vertex data, and it is received in the following
/// sequence:
/// - A [`VertexPositionPacket`], interpreted as the top-left corner of the Rectangle.
/// - If doing textured rectangle, a [`VertexUVPacket`].
/// - If doing variable sized rectangle, a [`VertexPositionPacket`] interpreted as the width and
/// height of the rectangle.
#[bitos(32)]
#[derive(Debug)]
pub struct RectangleInstr {
    #[bits(0..8)]
    color_r: u8,
    #[bits(8..16)]
    color_g: u8,
    #[bits(16..24)]
    color_b: u8,
    #[bits(24)]
    texture_mode: TextureMode,
    #[bits(25)]
    transparency_mode: TransparencyMode,
    #[bits(26)]
    textured: bool,
    #[bits(27..29)]
    rectangle_mode: RectangleMode,
}
