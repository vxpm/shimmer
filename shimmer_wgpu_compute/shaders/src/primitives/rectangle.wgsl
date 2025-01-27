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

fn rectangle_contains(rectangle: Rectangle, point: vec2u) -> bool {
    let coords = vec2i(point) - rectangle.top_left;
    if coords.x < 0 || coords.y < 0 || coords.x >= i32(rectangle.dimensions.x) || coords.y >= i32(rectangle.dimensions.y) {
        return false;
    }

    return true;
}

fn rectangle_uv(rectangle: Rectangle, point: vec2u) -> vec2u {
    return rectangle.top_left_uv + point - vec2u(rectangle.top_left);
}
