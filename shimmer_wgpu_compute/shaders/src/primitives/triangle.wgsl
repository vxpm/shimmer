//!include primitives
//!include shading
//!include texture

// A triangle primitive.
struct Triangle {
    // Vertices that make up this triangle, in counter-clockwise order.
    vertices: array<Vertex, 3>,
    // Shading mode of this triangle.
    shading_mode: ShadingMode,
    // Texture configuration of this triangle.
    texture: TextureConfig,
}

fn _triangle_cross_2d(a: vec2i, b: vec2i) -> i32 {
    return a.x * b.y - a.y * b.x;
}

fn _triangle_is_top_or_left_edge(edge: vec2i) -> bool {
    var is_top = edge.y == 0 && edge.x < 0;
    var is_left = edge.y < 0;
    return is_top || is_left;
}

fn triangle_barycentric_coords_of(triangle: Triangle, point: vec2i) -> vec3f {
    let ap = point - triangle.vertices[0].coords;
    let bp = point - triangle.vertices[1].coords;
    let cp = point - triangle.vertices[2].coords;

    let ab = triangle.vertices[1].coords - triangle.vertices[0].coords;
    let bc = triangle.vertices[2].coords - triangle.vertices[1].coords;
    let ca = triangle.vertices[0].coords - triangle.vertices[2].coords;

    let bias_a = select(0, 1, _triangle_is_top_or_left_edge(bc));
    let bias_b = select(0, 1, _triangle_is_top_or_left_edge(ca));
    let bias_c = select(0, 1, _triangle_is_top_or_left_edge(ab));

    let total = abs(f32(_triangle_cross_2d(ab, ca)));
    let wa = f32(_triangle_cross_2d(bc, bp) - bias_a);
    let wb = f32(_triangle_cross_2d(ca, cp) - bias_b);
    let wc = f32(_triangle_cross_2d(ab, ap) - bias_c);

    return vec3f(wa, wb, wc) / total;
}

fn triangle_color(triangle: Triangle, bary_coords: vec3f) -> RgbaNorm {
    let a = rgba8_normalize(triangle.vertices[0].color).value * bary_coords.x;
    let b = rgba8_normalize(triangle.vertices[1].color).value * bary_coords.y;
    let c = rgba8_normalize(triangle.vertices[2].color).value * bary_coords.z;

    return RgbaNorm(a + b + c);
}

fn triangle_uv(triangle: Triangle, bary_coords: vec3f) -> vec2u {
    let a = vec2f(triangle.vertices[0].uv) * bary_coords.x;
    let b = vec2f(triangle.vertices[1].uv) * bary_coords.y;
    let c = vec2f(triangle.vertices[2].uv) * bary_coords.z;

    return vec2u(floor(a + b + c));
}
