use super::{Rgba8, TexConfig};
use crate::gpu::cmd::rendering::ShadingMode;
use bitos::integer::i11;

/// A single triangle vertex.
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub color: Rgba8,
    pub x: i11,
    pub y: i11,
    pub u: u8,
    pub v: u8,
}

/// A triangle primitive.
#[derive(Debug, Clone, Copy)]
pub struct Triangle {
    pub vertices: [Vertex; 3],
    pub shading: ShadingMode,
    pub texconfig: Option<TexConfig>,
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
    pub texconfig: Option<TexConfig>,
}

/// A drawing primitive.
#[derive(Debug, Clone, Copy)]
pub enum Primitive {
    Triangle(Triangle),
    Rectangle(Rectangle),
}
