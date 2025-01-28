fn unorm_to_snorm(value: f32) -> f32 {
    return 2.0 * value - 1.0;
}

fn unorm_to_snorm_vec2(value: vec2f) -> vec2f {
    return 2.0 * value - vec2f(1.0);
}

fn snorm_to_unorm(value: f32) -> f32 {
    return (value + 1.0) / 2.0;
}

fn snorm_to_unorm_vec2(value: vec2f) -> vec2f {
    return (value + vec2f(1.0)) / 2.0;
}

fn unorm_to_u5(value: f32) -> u32 {
    return u32(value * 31.0) & 0x1Fu;
}
