//!include primitives
//!include shading
//!include texture

// A triangle primitive.
struct Triangle {
    // Vertices that make up this triangle, in counter-clockwise order.
    vertices: array<Vertex, 3>,
    // Shading mode of this triangle.
    shading_mode: ShadingMode,
    // Blending mode of this triangle.
    transparency_mode: TransparencyMode,
    // Texture configuration of this triangle.
    texture: TextureConfig,
}

fn _triangle_cross_2d(a: vec2f, b: vec2f) -> f32 {
    return a.x * b.y - a.y * b.x;
}

fn _triangle_is_top_or_left_edge(edge: vec2f) -> bool {
    var is_top = edge.y == 0.0 && edge.x < 0.0;
    var is_left = edge.y > 0.0;
    return is_top || is_left;
}

fn triangle_barycentric_coords_of(triangle: Triangle, point: vec2i) -> vec3f {
    let ap = vec2f(point - triangle.vertices[0].coords);
    let bp = vec2f(point - triangle.vertices[1].coords);
    let cp = vec2f(point - triangle.vertices[2].coords);

    let ab = vec2f(triangle.vertices[1].coords - triangle.vertices[0].coords);
    let bc = vec2f(triangle.vertices[2].coords - triangle.vertices[1].coords);
    let ca = vec2f(triangle.vertices[0].coords - triangle.vertices[2].coords);

    let bias_a = select(0.0, 0.01, _triangle_is_top_or_left_edge(bc));
    let bias_b = select(0.0, 0.01, _triangle_is_top_or_left_edge(ca));
    let bias_c = select(0.0, 0.01, _triangle_is_top_or_left_edge(ab));

    let total = abs(_triangle_cross_2d(ab, ca));
    let wa = _triangle_cross_2d(bc, bp) - bias_a;
    let wb = _triangle_cross_2d(ca, cp) - bias_b;
    let wc = _triangle_cross_2d(ab, ap) - bias_c;

    return vec3f(wa, wb, wc) / total;
}

fn triangle_color(triangle: Triangle, bary_coords: vec3f) -> RgbNorm {
    let a = rgb8_to_rgb_norm(triangle.vertices[0].color).value * bary_coords.x;
    let b = rgb8_to_rgb_norm(triangle.vertices[1].color).value * bary_coords.y;
    let c = rgb8_to_rgb_norm(triangle.vertices[2].color).value * bary_coords.z;

    return RgbNorm(a + b + c);
}

fn triangle_uv(triangle: Triangle, bary_coords: vec3f) -> vec2u {
    let a = vec2f(triangle.vertices[0].uv) * bary_coords.x;
    let b = vec2f(triangle.vertices[1].uv) * bary_coords.y;
    let c = vec2f(triangle.vertices[2].uv) * bary_coords.z;

    return vec2u(round(a + b + c));
}
