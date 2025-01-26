//!include primitives
//!include consts
//!include vram

@group(0) @binding(0)
var<storage, read_write> vram: array<u32>;
@group(1) @binding(0)
var<storage, read> triangles: array<Triangle>;

@compute @workgroup_size(8, 8, 1)
fn render(@builtin(global_invocation_id) global_id: vec3u) {
    var vram_coords = vec2u(global_id.x, global_id.y);

    for (var i: u32 = 0; i < arrayLength(&triangles); i += 1u) {
        var triangle = triangles[i];
        var bary_coords = triangle_barycentric_point_coords(triangle, vec2i(vram_coords));
        var is_inside = (bary_coords.x >= 0.0) && (bary_coords.y >= 0.0) && (bary_coords.z >= 0.0);
        if is_inside {
            var rgba_norm = triangle_color(triangle, bary_coords);
            vram_set_color_rgb5m(
                vram_coords,
                rgba_norm_to_rgb5m(rgba_norm),
            );
        }
    }
}
