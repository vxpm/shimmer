//!include primitives
//!include vram
//!include commands
//!include color

struct Config {
    drawing_area_top_left: vec2u,
    drawing_area_dimensions: vec2u,

    write_to_mask: u32,
    check_mask: u32,

    texwindow_mask: vec2u,
    texwindow_offset: vec2u,

    blending_mode: BlendingMode,
}

fn drawing_area_contains(coords: vec2u) -> bool {
    let relative = coords - config.drawing_area_top_left;
    return all((relative >= vec2u(0)) && (relative <= config.drawing_area_dimensions));
}

var<private> config: Config;

@group(0) @binding(0)
var<storage, read_write> vram: array<u32>;

@group(1) @binding(0)
var<storage, read> commands: array<Command>;
@group(1) @binding(1)
var<storage, read> configs: array<Config>;
@group(1) @binding(2)
var<storage, read> triangles: array<Triangle>;
@group(1) @binding(3)
var<storage, read> rectangles: array<Rectangle>;

fn render_triangle(triangle: Triangle, vram_coords: vec2u) -> bool {
    let info = triangle_barycentric_coords_of(triangle, vec2i(vram_coords));
    if !info.is_inside {
        return false;
    }

    if (config.check_mask > 0) && rgb5m_get_mask(vram_get_color_rgb5m(vram_coords)) {
        return false;
    }

    let bary_coords = info.weights;

    var color: Rgb5m;
    var pixel_transparency = true;
    switch triangle.texture.mode {
        case TEXTURE_MODE_NONE {
            if triangle.shading_mode == SHADING_MODE_GOURAUD {
                let rgb_norm = triangle_color(triangle, bary_coords);
                let dithered = rgb_norm_dither(vram_coords, rgb_norm);
                color = rgb_norm_to_rgb5m(dithered);
            } else {
                let rgb_norm = triangle_color(triangle, bary_coords);
                color = rgb_norm_to_rgb5m(rgb_norm);
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

            pixel_transparency = rgb5m_get_mask(texel);
        }
        default: {
            color = RGB5M_PLACEHOLDER;
        }
    }

    if triangle.transparency_mode == TRANSPARENCY_MODE_TRANSPARENT && pixel_transparency {
        let bg = rgb5m_to_rgb_norm(vram_get_color_rgb5m(vram_coords));
        let fg = rgb5m_to_rgb_norm(color);
        let blended = rgb_norm_blend(config.blending_mode, bg, fg);

        color = rgb_norm_to_rgb5m(blended);
    }

    if config.write_to_mask > 0 {
        color = rgb5m_set_mask(color);
    }

    vram_set_color_rgb5m(vram_coords, color);
    return true;
}

fn render_rectangle(rectangle: Rectangle, vram_coords: vec2u) -> bool {
    if !rectangle_contains(rectangle, vram_coords) {
        return false;
    }

    if (config.check_mask > 0) && rgb5m_get_mask(vram_get_color_rgb5m(vram_coords)) {
        return false;
    }

    var color: Rgb5m;
    var pixel_transparency = true;
    switch rectangle.texture.mode {
        case TEXTURE_MODE_NONE {
            let rgb_norm = rgb8_to_rgb_norm(rectangle.top_left.color);
            color = rgb_norm_to_rgb5m(rgb_norm);
        }
        case TEXTURE_MODE_LUT4, TEXTURE_MODE_LUT8, TEXTURE_MODE_FULL {
            let uv = rectangle_uv(rectangle, vram_coords);
            let texel = texture_texel(rectangle.texture, uv);

            if texel.value == RGB5M_TRANSPARENT.value {
                color = vram_get_color_rgb5m(vram_coords);
            } else {
                color = texel;
            }

            pixel_transparency = rgb5m_get_mask(texel);
        }
        default: {
            color = RGB5M_PLACEHOLDER;
        }
    }

    if rectangle.transparency_mode == TRANSPARENCY_MODE_TRANSPARENT && pixel_transparency {
        let bg = rgb5m_to_rgb_norm(vram_get_color_rgb5m(vram_coords));
        let fg = rgb5m_to_rgb_norm(color);
        let blended = rgb_norm_blend(config.blending_mode, bg, fg);

        color = rgb_norm_to_rgb5m(blended);
    }

    if config.write_to_mask > 0 {
        color = rgb5m_set_mask(color);
    }

    vram_set_color_rgb5m(vram_coords, color);
    return true;
}

@compute @workgroup_size(8, 8, 1)
fn render(@builtin(global_invocation_id) global_id: vec3u) {
    let vram_coords = vec2u(global_id.x, global_id.y);

    config = configs[0];
    var config_index = 1u;
    var triangle_index = 0u;
    var rectangle_index = 0u;

    for (var i: u32 = 0; i < arrayLength(&commands); i += 1u) {
        let command = commands[i];
        switch command {
            case COMMAND_CONFIG {
                config = configs[config_index];
                config_index += 1;
            }
            case COMMAND_TRIANGLE {
                if drawing_area_contains(vram_coords) {
                    render_triangle(triangles[triangle_index], vram_coords);
                }
                triangle_index += 1u;
            }
            case COMMAND_RECTANGLE {
                if drawing_area_contains(vram_coords) {
                    render_rectangle(rectangles[rectangle_index], vram_coords);
                }
                rectangle_index += 1u;
            }
            default: {
                vram_set_color_rgb5m(vram_coords, RGB5M_PLACEHOLDER);
                return;
            } 
            case COMMAND_FINISH {
                return;
            }
        }
    }
}
