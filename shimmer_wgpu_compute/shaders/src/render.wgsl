//!include primitives
//!include consts
//!include vram
//!include commands

@group(0) @binding(0)
var<storage, read_write> vram: array<u32>;

@group(1) @binding(0)
var<storage, read> commands: array<u32>;
@group(1) @binding(1)
var<storage, read> triangles: array<Triangle>;

@compute @workgroup_size(8, 8, 1)
fn render(@builtin(global_invocation_id) global_id: vec3u) {
    var vram_coords = vec2u(global_id.x, global_id.y);

    var triangle_index = 0;
    for (var i: u32 = 0; i < arrayLength(&commands); i += 1u) {
        let command = commands[i];
        switch command {
            case COMMAND_BARRIER {
                storageBarrier();
            }
            case COMMAND_TRIANGLE {
                var triangle = triangles[triangle_index];

                var bary_coords = triangle_barycentric_coords_of(triangle, vec2i(vram_coords));
                var is_inside = (bary_coords.x >= 0.0) && (bary_coords.y >= 0.0) && (bary_coords.z >= 0.0);

                if is_inside {
                    let rgba_norm = triangle_color(triangle, bary_coords);
                    let dithered = rgba_norm_dither(vram_coords, rgba_norm);
                    let color = rgba_norm_to_rgb5m(dithered);

                    vram_set_color_rgb5m(
                        vram_coords,
                        color
                    );
                }

                triangle_index += 1;
            }
            default: {}
        }
    }
}
