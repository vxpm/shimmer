//! The rendering interface for renderer implementations.

use super::{
    HorizontalResolution, VerticalResolution,
    cmd::{
        environment::TexPage,
        rendering::{Clut, ShadingMode},
    },
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
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}

/// A single triangle vertex.
#[derive(Debug, Clone, Copy, Immutable, FromBytes, IntoBytes)]
#[repr(C)]
pub struct Vertex {
    pub color: Rgba8,
    pub x: i11,
    pub y: i11,
    pub u: u8,
    pub v: u8,
    pub _padding: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct TextureConfig {
    pub clut: Clut,
    pub texpage: TexPage,
}

#[derive(Debug, Clone)]
pub struct Triangle {
    pub vertices: [Vertex; 3],
    pub shading: ShadingMode,
    pub texture: Option<TextureConfig>,
}

#[derive(Debug, Clone)]
pub struct Rectangle {
    pub color: Rgba8,
    pub x: i11,
    pub y: i11,
    pub u: u8,
    pub v: u8,
    pub width: u10,
    pub height: u10,
    pub texture: Option<TextureConfig>,
}

/// A data copy to VRAM.
#[derive(Debug, Clone)]
pub struct CopyToVram {
    pub x: u10,
    pub y: u10,
    pub width: u10,
    pub height: u10,
    pub data: Vec<u8>,
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
#[derive(Debug, Clone)]
pub enum Command {
    // Configuration
    SetDisplayTopLeft(DisplayTopLeft),
    SetDisplayResolution(DisplayResolution),

    // Control
    Vsync,

    // Copy data
    CopyToVram(CopyToVram),

    // Draw stuff
    DrawTriangle(Triangle),
    DrawRectangle(Rectangle),
}
