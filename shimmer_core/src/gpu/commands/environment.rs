use bitos::{
    bitos,
    integer::{u1, u4, u5, u9, u10, u11},
};

/// The shading mode of a rendering command.
#[bitos(2)]
#[derive(Debug, PartialEq, Eq)]
pub enum SemiTransparencyMode {
    /// Final Color = Old / 2 + New / 2
    Half = 0,
    /// Final Color = Old + New
    Add = 1,
    /// Final Color = Old - New
    Sub = 2,
    /// Final Color = Old + New / 4
    Quarter = 3,
}

/// The bit depth of the texture page.
#[bitos(2)]
#[derive(Debug, PartialEq, Eq)]
pub enum TexturePageDepth {
    Nibble = 0,
    Byte = 1,
    /// 15 Bit
    Full = 2,
    Reserved = 3,
}

/// The compression mode of colors.
#[bitos(1)]
#[derive(Debug, PartialEq, Eq)]
pub enum CompressionMode {
    /// Strip LSBs.
    Strip = 0,
    /// Perform dithering.
    Dither = 1,
}

/// A drawing settings command.
#[bitos(32)]
#[derive(Debug)]
pub struct DrawingSettingsCmd {
    #[bits(0..4)]
    pub texpage_x_base: u4,
    #[bits(4)]
    pub texpage_y_base: u1,
    #[bits(5..7)]
    pub semi_transparency_mode: SemiTransparencyMode,
    #[bits(7..9)]
    pub texpage_depth: TexturePageDepth,
    #[bits(9)]
    pub compression_mode: CompressionMode,
    #[bits(10)]
    pub enable_drawing_to_display: bool,
    #[bits(11)]
    pub texpage_y_base_2: u1,
    #[bits(12)]
    pub textured_rect_flip_x: bool,
    #[bits(13)]
    pub textured_rect_flip_y: bool,
}

/// A texture window settings command.
#[bitos(32)]
#[derive(Debug)]
pub struct TextureWindowSettingsCmd {
    #[bits(0..5)]
    tex_window_mask_x: u5,
    #[bits(5..10)]
    tex_window_mask_y: u5,
    #[bits(10..15)]
    tex_window_offset_x: u5,
    #[bits(15..20)]
    tex_window_offset_y: u5,
}

/// A drawing area corner command.
#[bitos(32)]
#[derive(Debug)]
pub struct DrawingAreaCornerCmd {
    #[bits(0..10)]
    x: u10,
    #[bits(10..19)]
    y: u9,
}

/// A drawing offset command.
#[bitos(32)]
#[derive(Debug)]
pub struct DrawingOffsetCmd {
    #[bits(0..11)]
    unsigned_x: u11,
    #[bits(11..22)]
    unsigned_y: u11,
}

/// A drawing offset command.
#[bitos(32)]
#[derive(Debug)]
pub struct MaskSettingsCmd {
    // TODO: define this
}
