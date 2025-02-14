//!include color

const VRAM_WIDTH: u32 = 1024;
const VRAM_HEIGHT: u32 = 512;
const VRAM_WIDTH_BYTES: u32 = 2 * VRAM_WIDTH;
const VRAM_HEIGHT_BYTES: u32 = 2 * VRAM_HEIGHT;

fn vram_get_color_rgb5m(coords: vec2u) -> Rgb5m {
    var index = (coords.y % VRAM_HEIGHT) * VRAM_WIDTH_BYTES + (coords.x % VRAM_WIDTH) * 2;

    var result = 0u;
    result = insertBits(result, vram[index], 0u, 8u);
    result = insertBits(result, vram[index + 1], 8u, 8u);

    return Rgb5m(result);
}

fn vram_set_color_rgb5m(coords: vec2u, rgb5m: Rgb5m) {
    var index = (coords.y % VRAM_HEIGHT) * VRAM_WIDTH_BYTES + (coords.x % VRAM_WIDTH) * 2;

    vram[index] = extractBits(rgb5m.value, 0u, 8u);
    vram[index + 1] = extractBits(rgb5m.value, 8u, 8u);
}
