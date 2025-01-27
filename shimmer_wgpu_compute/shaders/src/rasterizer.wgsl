//!include primitives
//!include vram
//!include commands
//!include color

@group(0) @binding(0)
var<storage, read_write> vram: array<u32>;

@group(1) @binding(0)
var<storage, read> commands: array<Command>;
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
                    var color: Rgb5m;
                    switch triangle.texture.mode {
                        case TEXTURE_MODE_NONE {
                            if triangle.shading_mode == SHADING_MODE_GOURAUD {
                                let rgba_norm = triangle_color(triangle, bary_coords);
                                let dithered = rgba_norm_dither(vram_coords, rgba_norm);
                                color = rgba_norm_to_rgb5m(dithered);
                            } else {
                                let rgba_norm = triangle_color(triangle, bary_coords);
                                color = rgba_norm_to_rgb5m(rgba_norm);
                            }
                        }
                        case TEXTURE_MODE_LUT4 {
                            let uv = triangle_uv(triangle, bary_coords);
                            let texel = texture_texel(triangle.texture, uv);

                            if texel.value == RGB5M_TRANSPARENT.value {
                                color = vram_get_color_rgb5m(vram_coords);
                            } else {
                                color = texel;
                            }
                        }
                        default: {
                            color = RGB5M_PLACEHOLDER;
                        }
                    }

                    vram_set_color_rgb5m(vram_coords, color);
                }

                triangle_index += 1;
            }
            default: {}
        }
    }
}
