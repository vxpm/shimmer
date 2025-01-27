//!include color
//!include primitives/triangle

// A single vertex of a primitive.
struct Vertex {
    coords: vec2i,
    color: Rgba8,
    uv: vec2u,
}
