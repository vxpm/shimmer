// A triangle primitive.
struct Triangle {
    /// Vertices that make up this triangle, in clockwise order.
    vertices: array<vec2u, 3>,
}

fn triangle_barycentric_point_coords(triangle: Triangle, point: vec2u) -> vec3f {
    let a = vec2f(triangle.vertices[0]);
    let b = vec2f(triangle.vertices[1]);
    let c = vec2f(triangle.vertices[2]);
    let p = vec2f(point);

    let v0 = b - a;
    let v1 = c - a;
    let v2 = p - a;

    let d00 = dot(v0, v0);
    let d01 = dot(v0, v1);
    let d11 = dot(v1, v1);
    let d20 = dot(v2, v0);
    let d21 = dot(v2, v1);
    let denom = d00 * d11 - d01 * d01;
    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;

    return vec3f(1.0 - v - w, v, w);
}
