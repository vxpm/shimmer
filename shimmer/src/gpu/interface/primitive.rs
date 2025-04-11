use super::{Rgba8, TexConfig};
use bitos::integer::i11;
use shimmer_core::gpu::cmd::rendering::{ShadingMode, TransparencyMode};

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
    pub transparency: TransparencyMode,
    pub texconfig: Option<TexConfig>,
}

impl Triangle {
    pub fn is_too_big(&self) -> bool {
        let dist = |a: Vertex, b: Vertex| {
            a.x.value().abs_diff(b.x.value()) > 1023 || a.y.value().abs_diff(b.y.value()) > 511
        };

        dist(self.vertices[0], self.vertices[1])
            || dist(self.vertices[1], self.vertices[2])
            || dist(self.vertices[2], self.vertices[1])
    }
}

/// A rectangle primitive.
#[derive(Debug, Clone, Copy)]
pub struct Rectangle {
    pub top_left: Vertex,
    pub width: u16,
    pub height: u16,
    pub transparency: TransparencyMode,
    pub texconfig: Option<TexConfig>,
}

impl Rectangle {
    pub fn is_too_big(&self) -> bool {
        self.width > 1023 || self.height > 511
    }
}

/// A drawing primitive.
#[derive(Debug, Clone, Copy)]
pub enum Primitive {
    Triangle(Triangle),
    Rectangle(Rectangle),
}
