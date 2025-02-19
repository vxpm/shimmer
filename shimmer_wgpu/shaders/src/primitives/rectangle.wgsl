//!include primitives
//!include texture

// A triangle primitive.
struct Rectangle {
    top_left: vec2i,
    top_left_uv: vec2u,
    dimensions: vec2u,
    color: Rgba8,
    texture: TextureConfig,
}

fn rectangle_contains(rect: Rectangle, point: vec2u) -> bool {
    let relative = vec2i(point) - rect.top_left;
    return all(relative >= vec2i(0) && relative < vec2i(rect.dimensions));
}

fn rectangle_uv(rect: Rectangle, point: vec2u) -> vec2u {
    let relative = vec2i(point) - rect.top_left;
    return rect.top_left_uv + vec2u(relative);
}
