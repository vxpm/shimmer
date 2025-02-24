alias TextureMode = u32;

const TEXTURE_MODE_NONE: TextureMode = 0;
const TEXTURE_MODE_LUT4: TextureMode = 1;
const TEXTURE_MODE_LUT8: TextureMode = 2;
const TEXTURE_MODE_FULL: TextureMode = 3;

struct TextureConfig {
    mode: TextureMode,
    clut: vec2u,
    texpage: vec2u,
}

fn apply_texwindow(coords: vec2u) -> vec2u {
    return vec2u(
        (coords.x & ~(config.texwindow_mask.x * 8u)) | ((config.texwindow_offset.x & config.texwindow_mask.x) * 8),
        (coords.y & ~(config.texwindow_mask.y * 8u)) | ((config.texwindow_offset.y & config.texwindow_mask.y) * 8),
    );
}

fn texture_texel(config: TextureConfig, uv: vec2u) -> Rgb5m {
    switch config.mode {
        case TEXTURE_MODE_LUT4 {
            var texpage_vram_coords = config.texpage + uv / vec2u(4, 1);
            texpage_vram_coords = apply_texwindow(texpage_vram_coords);

            var texel_index_group = vram_get_color_rgb5m(texpage_vram_coords);
            var clut_index = extractBits(texel_index_group.value, 4 * (uv.x % 4), 4u);
            var texel = vram_get_color_rgb5m(config.clut + vec2u(clut_index, 0));

            return texel;
        }
        case TEXTURE_MODE_LUT8 {
            var texpage_vram_coords = config.texpage + uv / vec2u(2, 1);
            texpage_vram_coords = apply_texwindow(texpage_vram_coords);

            var texel_index_group = vram_get_color_rgb5m(texpage_vram_coords);
            var clut_index = extractBits(texel_index_group.value, 8 * (uv.x % 2), 8u);
            var texel = vram_get_color_rgb5m(config.clut + vec2u(clut_index, 0));

            return texel;
        }
        case TEXTURE_MODE_FULL {
            var texpage_vram_coords = config.texpage + uv;
            texpage_vram_coords = apply_texwindow(texpage_vram_coords);

            var texel = vram_get_color_rgb5m(texpage_vram_coords);
            return texel;
        }
        default: {
            return RGB5M_PLACEHOLDER;
        }
    }
}
