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

fn texture_texel(config: TextureConfig, uv: vec2u) -> Rgb5m {
    switch config.mode {
        case TEXTURE_MODE_LUT4 {
            var texpage_vram_coords = config.texpage + uv / vec2u(4, 1);
            var texel_index_group = vram_get_color_rgb5m(texpage_vram_coords);

            var clut_index = extractBits(texel_index_group.value, 4 * (uv.x % 4), 4u);
            var texel = vram_get_color_rgb5m(config.clut + vec2u(clut_index, 0));

            return texel;
        }
        default: {
            return RGB5M_PLACEHOLDER;
        }
    }
}
