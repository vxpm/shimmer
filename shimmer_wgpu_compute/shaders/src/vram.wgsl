//!include consts

fn vram_get_color_rgb5m(coords: vec2u) -> Rgb5m {
    var index = coords.y * VRAM_WIDTH_BYTES + coords.x * 2;

    var result = 0u;
    result = insertBits(result, vram[index], 0u, 8u);
    result = insertBits(result, vram[index + 1], 8u, 8u);

    return Rgb5m(result);
}

fn vram_set_color_rgb5m(coords: vec2u, rgb5m: Rgb5m) {
    var index = coords.y * VRAM_WIDTH_BYTES + coords.x * 2;

    vram[index] = extractBits(rgb5m.value, 0u, 8u);
    vram[index + 1] = extractBits(rgb5m.value, 8u, 8u);
}
