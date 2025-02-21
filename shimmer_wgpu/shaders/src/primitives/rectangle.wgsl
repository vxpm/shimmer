//!include primitives
//!include texture

// A triangle primitive.
struct Rectangle {
    top_left: Vertex,
    dimensions: vec2u,
    transparency_mode: TransparencyMode,
    texture: TextureConfig,
}

fn rectangle_contains(rect: Rectangle, point: vec2u) -> bool {
    let relative = vec2i(point) - rect.top_left.coords;
    return all(relative >= vec2i(0) && relative < vec2i(rect.dimensions));
}

fn rectangle_uv(rect: Rectangle, point: vec2u) -> vec2u {
    let relative = vec2i(point) - rect.top_left.coords;
    return rect.top_left.uv + vec2u(relative);
}
