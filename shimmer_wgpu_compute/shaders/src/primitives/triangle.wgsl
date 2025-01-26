//!include primitives

// A triangle primitive.
struct Triangle {
    /// Vertices that make up this triangle, in counter-clockwise order.
    vertices: array<Vertex, 3>,
}

fn cross_2d(a: vec2i, b: vec2i) -> i32 {
    return a.x * b.y - a.y * b.x;
}

fn triangle_barycentric_point_coords(triangle: Triangle, point: vec2i) -> vec3f {
    let ap = point - triangle.vertices[0].coords;
    let bp = point - triangle.vertices[1].coords;
    let cp = point - triangle.vertices[2].coords;

    let ab = triangle.vertices[1].coords - triangle.vertices[0].coords;
    let bc = triangle.vertices[2].coords - triangle.vertices[1].coords;
    let ca = triangle.vertices[0].coords - triangle.vertices[2].coords;

    let total = abs(f32(cross_2d(ab, ca)));
    let wc = f32(cross_2d(ab, ap)) / total;
    let wa = f32(cross_2d(bc, bp)) / total;
    let wb = f32(cross_2d(ca, cp)) / total;

    return vec3f(wa, wb, wc);
}

fn triangle_color(triangle: Triangle, bary_coords: vec3f) -> RgbaNorm {
    let a = rgba8_normalize(triangle.vertices[0].color).value * bary_coords.x;
    let b = rgba8_normalize(triangle.vertices[1].color).value * bary_coords.y;
    let c = rgba8_normalize(triangle.vertices[2].color).value * bary_coords.z;

    return RgbaNorm(a + b + c);
}
