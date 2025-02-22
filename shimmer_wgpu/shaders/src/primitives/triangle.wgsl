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

fn _triangle_cross_2d(a: vec2i, b: vec2i) -> i32 {
    return a.x * b.y - a.y * b.x;
}

fn _triangle_is_top_or_left_edge(edge: vec2i) -> bool {
    var is_top = edge.y == 0 && edge.x > 0;
    var is_left = edge.y < 0;
    return is_left || is_top;
}

struct BaryCoords {
    is_inside: bool,
    weights: vec3f,
}

fn triangle_barycentric_coords_of(triangle: Triangle, point: vec2i) -> BaryCoords {
    let ap = point - triangle.vertices[0].coords;
    let bp = point - triangle.vertices[1].coords;
    let cp = point - triangle.vertices[2].coords;

    let ab = triangle.vertices[1].coords - triangle.vertices[0].coords;
    let bc = triangle.vertices[2].coords - triangle.vertices[1].coords;
    let ca = triangle.vertices[0].coords - triangle.vertices[2].coords;

    let areas = vec3i(
        _triangle_cross_2d(bc, bp),
        _triangle_cross_2d(ca, cp),
        _triangle_cross_2d(ab, ap),
    );

    let is_top_left = vec3<bool>(
        _triangle_is_top_or_left_edge(bc),
        _triangle_is_top_or_left_edge(ca),
        _triangle_is_top_or_left_edge(ab),
    );

    var is_inside = all(areas >= vec3i(0));
    if areas.x == 0 && !is_top_left.x {
        is_inside = false;
    }

    if areas.y == 0 && !is_top_left.y {
        is_inside = false;
    }

    if areas.z == 0 && !is_top_left.z {
        is_inside = false;
    }

    let total = abs(_triangle_cross_2d(ab, ca));
    let weights = vec3f(areas) / f32(total);
    return BaryCoords(is_inside, weights);
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
