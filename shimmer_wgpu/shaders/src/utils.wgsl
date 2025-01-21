const RGB5M_TRANSPARENT: u32 = 0;
const RGB5M_PLACEHOLDER: u32 = 0xDEAD;

fn unorm_to_snorm(value: f32) -> f32 {
    return 2.0 * value - 1.0;
}

fn snorm_to_unorm(value: f32) -> f32 {
    return (value + 1.0) / 2.0;
}

fn snorm_to_unorm_vec2(value: vec2f) -> vec2f {
    return (value + vec2f(1.0)) / vec2f(2.0);
}

fn unorm_to_u5(value: f32) -> u32 {
    return u32(value * 31.0) & 0x1Fu;
}

fn unorm_rgba_to_rgb5m(rgba: vec4<f32>) -> u32 {
    var r = unorm_to_u5(rgba.r);
    var g = unorm_to_u5(rgba.g);
    var b = unorm_to_u5(rgba.b);
    return r | (g << 5) | (b << 10);
}
