//!include color
//!include primitives/triangle
//!include primitives/rectangle

alias BlendingMode = u32;
const BLENDING_MODE_OPAQUE: BlendingMode = 0;
const BLENDING_MODE_TRANSPARENT: BlendingMode = 1;

// A single vertex of a primitive.
struct Vertex {
    coords: vec2i,
    color: Rgb8,
    uv: vec2u,
}

