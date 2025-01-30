//! The rendering interface for renderer implementations.

use super::{
    HorizontalResolution, VerticalResolution,
    cmd::rendering::ShadingMode,
    texture::{Clut, TexPage},
};
use bitos::integer::{i11, u9, u10};
use zerocopy::{FromBytes, Immutable, IntoBytes};

/// Full 32-bit RGBA color.
#[derive(Debug, Clone, Copy, Immutable, FromBytes, IntoBytes, Default)]
#[repr(C)]
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

/// A single triangle vertex.
#[derive(Debug, Clone, Copy, Immutable, FromBytes, IntoBytes)]
pub struct Vertex {
    pub color: Rgba8,
    pub x: i11,
    pub y: i11,
    pub u: u8,
    pub v: u8,
}

/// Texture configuration.
#[derive(Debug, Clone, Copy, Default)]
pub struct TextureConfig {
    pub clut: Clut,
    pub texpage: TexPage,
}

/// A triangle primitive.
#[derive(Debug, Clone, Copy)]
pub struct Triangle {
    pub vertices: [Vertex; 3],
    pub shading: ShadingMode,
    pub texture: Option<TextureConfig>,
}

/// A rectangle primitive.
#[derive(Debug, Clone, Copy)]
pub struct Rectangle {
    pub color: Rgba8,
    pub x: i11,
    pub y: i11,
    pub u: u8,
    pub v: u8,
    pub width: u16,
    pub height: u16,
    pub texture: Option<TextureConfig>,
}

/// A data copy to VRAM.
#[derive(Debug, Clone)]
pub struct CopyToVram {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub data: Vec<u8>,
}

/// A data copy from VRAM.
#[derive(Debug)]
pub struct CopyFromVram {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub response: oneshot::Sender<Vec<u8>>,
}

/// Top-Left position of the display.
#[derive(Debug, Clone)]
pub struct DisplayTopLeft {
    pub x: u10,
    pub y: u9,
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
    SetDisplayTopLeft(DisplayTopLeft),
    SetDisplayResolution(DisplayResolution),

    // Control
    VBlank,

    // Copy data
    CopyToVram(CopyToVram),
    CopyFromVram(CopyFromVram),

    // Draw stuff
    DrawTriangle(Triangle),
    DrawRectangle(Rectangle),
}

/// Renderer interface.
pub trait Renderer: Send + Sync {
    /// Executes a single renderer command. This method should execute as quickly as possible in
    /// order to not disturb emulator timing. It is recommended to offload the rendering to another
    /// thread.
    fn exec(&mut self, command: Command);
}
