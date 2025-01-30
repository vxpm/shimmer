//! Environment commands.

use crate::gpu::{
    CompressionMode,
    texture::{TexPage, TexWindow},
};
use bitos::{
    bitos,
    integer::{i11, u9, u10},
};

/// A drawing settings command.
#[bitos(32)]
#[derive(Debug, Clone)]
pub struct DrawingSettingsCmd {
    #[bits(0..9)]
    pub texpage: TexPage,
    #[bits(9)]
    pub compression_mode: CompressionMode,
    #[bits(10)]
    pub enable_drawing_to_display: bool,
    #[bits(11)]
    pub texture_disable: bool,
    #[bits(12)]
    pub textured_rect_flip_x: bool,
    #[bits(13)]
    pub textured_rect_flip_y: bool,
}

/// A drawing offset command.
#[bitos(32)]
#[derive(Debug)]
pub struct TextureWindowSettingsCmd {
    #[bits(..20)]
    texwindow: TexWindow,
}

/// A drawing area corner command.
#[bitos(32)]
#[derive(Debug, Clone)]
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
    x: i11,
    #[bits(11..22)]
    y: i11,
}

/// A drawing offset command.
#[bitos(32)]
#[derive(Debug)]
pub struct MaskSettingsCmd {
    // TODO: define this
}
