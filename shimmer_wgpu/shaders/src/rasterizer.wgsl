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
@group(1) @binding(2)
var<storage, read> rectangles: array<Rectangle>;

fn render_triangle(triangle: Triangle, vram_coords: vec2u) {
    var bary_coords = triangle_barycentric_coords_of(triangle, vec2i(vram_coords));
    var is_inside = (bary_coords.x >= 0.0) && (bary_coords.y >= 0.0) && (bary_coords.z >= 0.0);

    if !is_inside {
        return;
    }

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
        case TEXTURE_MODE_LUT4, TEXTURE_MODE_LUT8, TEXTURE_MODE_FULL {
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

fn render_rectangle(rectangle: Rectangle, vram_coords: vec2u) {
    if !rectangle_contains(rectangle, vram_coords) {
        return;
    }

    var color: Rgb5m;
    switch rectangle.texture.mode {
        case TEXTURE_MODE_NONE {
            let rgba_norm = rgba8_normalize(rectangle.color);
            color = rgba_norm_to_rgb5m(rgba_norm);
        }
        case TEXTURE_MODE_LUT4, TEXTURE_MODE_LUT8, TEXTURE_MODE_FULL {
            let uv = rectangle_uv(rectangle, vram_coords);
            let texel = texture_texel(rectangle.texture, uv);

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

@compute @workgroup_size(8, 8, 1)
fn render(@builtin(global_invocation_id) global_id: vec3u) {
    let vram_coords = vec2u(global_id.x, global_id.y);

    var triangle_index = 0u;
    var rectangle_index = 0u;
    for (var i: u32 = 0; i < arrayLength(&commands); i += 1u) {
        let command = commands[i];
        switch command {
            case COMMAND_TRIANGLE {
                render_triangle(triangles[triangle_index], vram_coords);
                triangle_index += 1u;
            }
            case COMMAND_RECTANGLE {
                render_rectangle(rectangles[rectangle_index], vram_coords);
                rectangle_index += 1u;
            }
            default: {
                vram_set_color_rgb5m(vram_coords, RGB5M_PLACEHOLDER);
                return;
            } 
        }
    }
}
