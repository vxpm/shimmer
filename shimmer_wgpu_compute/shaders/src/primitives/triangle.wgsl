//!include primitives

// A triangle primitive.
struct Triangle {
    /// Vertices that make up this triangle, in clockwise order.
    vertices: array<Vertex, 3>,
}

fn triangle_barycentric_point_coords(triangle: Triangle, point: vec2u) -> vec2f {
    var _a = vec2f(triangle.vertices[0].coords);
    var _b = vec2f(triangle.vertices[1].coords);
    var _c = vec2f(triangle.vertices[2].coords);
    var _p = vec2f(point);

    var c = _c - _a;
    var b = _b - _a;
    var p = _p - _a;

    var cc = dot(c, c);
    var bc = dot(b, c);
    var pc = dot(c, p);
    var bb = dot(b, b);
    var pb = dot(b, p);

    var denom = cc * bb - bc * bc;
    var u = (bb * pc - bc * pb) / denom;
    var v = (cc * pb - bc * pc) / denom;

    return vec2f(u, v);
}
