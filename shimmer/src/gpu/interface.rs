//! The interface for renderer implementations.

pub mod primitive;

use bitos::integer::{u9, u10, u11};
use shimmer_core::gpu::{
    HorizontalResolution, VerticalResolution,
    texture::{Clut, TexPage, TexWindow},
};

pub use primitive::*;

/// VRAM coordinates.
#[derive(Debug, Clone)]
pub struct VramCoords {
    pub x: u10,
    pub y: u9,
}

/// VRAM dimensions.
#[derive(Debug, Clone)]
pub struct VramDimensions {
    pub width: u11,
    pub height: u10,
}

/// 32-bit RGBA color.
#[derive(Debug, Clone, Copy, Default)]
pub struct Rgba8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba8 {
    #[inline(always)]
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}

/// Texture configuration.
#[derive(Debug, Clone, Copy, Default)]
pub struct TexConfig {
    pub clut: Clut,
    pub texpage: TexPage,
    pub texwindow: TexWindow,
}

/// A data copy to VRAM.
#[derive(Debug, Clone)]
pub struct CopyToVram {
    pub coords: VramCoords,
    pub dimensions: VramDimensions,
    pub data: Vec<u8>,
}

/// A data copy from VRAM.
#[derive(Debug)]
pub struct CopyFromVram {
    pub coords: VramCoords,
    pub dimensions: VramDimensions,
    pub response: oneshot::Sender<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct DrawingArea {
    pub coords: VramCoords,
    pub dimensions: VramDimensions,
}

/// Top-Left position of the display.
#[derive(Debug, Clone)]
pub struct DisplayResolution {
    pub horizontal: HorizontalResolution,
    pub vertical: VerticalResolution,
}

/// A renderer command.
#[derive(Debug)]
pub enum Command {
    // Configuration
    SetDrawingArea(DrawingArea),
    SetDisplayTopLeft(VramCoords),
    SetDisplayResolution(DisplayResolution),

    // Control
    VBlank,

    // Copy data
    CopyToVram(CopyToVram),
    CopyFromVram(CopyFromVram),

    // Draw
    Draw { primitive: Primitive },
}

/// Renderer interface.
pub trait Renderer: Send + Sync {
    /// Executes a single renderer command. This method should execute as quickly as possible in
    /// order to not disturb emulator timing. It is recommended to offload the rendering to another
    /// thread.
    fn exec(&mut self, command: Command);
}
