//!include color
//!include primitives/triangle
//!include primitives/rectangle

alias TransparencyMode = u32;
const TRANSPARENCY_MODE_OPAQUE: TransparencyMode = 0;
const TRANSPARENCY_MODE_TRANSPARENT: TransparencyMode = 1;

// A single vertex of a primitive.
struct Vertex {
    coords: vec2i,
    color: Rgb8,
    uv: vec2u,
}

