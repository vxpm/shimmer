//!include color
//!include primitives/triangle

// A single vertex of a primitive.
struct Vertex {
    color: Rgba8,
    vram_x: u32,
    vram_y: u32,
    padding0: u32,
    padding1: u32,
}
