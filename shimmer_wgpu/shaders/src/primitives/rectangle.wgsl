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
    let spoint = vec2i(point);
    let sdimensions = vec2i(rect.dimensions);
    let horizontal = (rect.top_left.x <= spoint.x) && (spoint.x < rect.top_left.x + sdimensions.x);
    let vertical = (rect.top_left.y <= spoint.y) && (spoint.y < rect.top_left.y + sdimensions.y);

    return horizontal && vertical;
}

fn rectangle_uv(rectangle: Rectangle, point: vec2u) -> vec2u {
    return rectangle.top_left_uv + point - vec2u(rectangle.top_left);
}
