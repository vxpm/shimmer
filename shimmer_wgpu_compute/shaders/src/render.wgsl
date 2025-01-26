//!include primitives
//!include consts
//!include vram

@group(0) @binding(0)
var<storage, read_write> vram: array<u32>;
@group(1) @binding(0)
var<storage, read> triangles: array<Triangle>;

@compute @workgroup_size(8, 8, 1)
fn render(@builtin(global_invocation_id) global_id: vec3u) {
    let vram_coords = vec2u(global_id.x, global_id.y);

    for (var i: u32 = 0; i < arrayLength(&triangles); i += 1u) {
        // vram_set_color_rgb5m(
        //     vram_coords,
        //     rgba_norm_to_rgb5m(RgbaNorm(vec4f(
        //         f32(vram_coords.x % 256) / 255.0,
        //         f32(vram_coords.y % 256) / 255.0,
        //         0.0,
        //         1.0
        //     )))
        // );

        let bary_coords = triangle_barycentric_point_coords(triangles[i], vram_coords);
        let is_inside = (bary_coords.x >= 0.0) && (bary_coords.y >= 0.0) && (bary_coords.x + bary_coords.y) < 1.0;

        if is_inside {
            vram_set_color_rgb5m(
                vram_coords,
                rgba_norm_to_rgb5m(RgbaNorm(vec4f(
                    f32(vram_coords.x % 256) / 255.0,
                    f32(vram_coords.y % 256) / 255.0,
                    0.0,
                    1.0
                )))
            );
        }
    }
}
