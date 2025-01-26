//!include utils

struct Rgb5m {
    value: u32,
}

fn rgb5m_to_rgba_norm(rgb5m: Rgb5m) -> RgbaNorm {
    var rgb5 = vec3u(
        extractBits(rgb5m.value, 0u, 5u),
        extractBits(rgb5m.value, 5u, 5u),
        extractBits(rgb5m.value, 10u, 5u)
    );

    return RgbaNorm(vec4f(vec3f(rgb5) / 32.0, 1.0));
}

// A normalized RGBA color.
struct RgbaNorm {
    value: vec4f,
}

fn rgba_norm_dither(coords: vec2u, rgba: RgbaNorm) -> RgbaNorm {
    const dither: mat4x4f = mat4x4f(
        -4.0, 0.0, -3.0, 1.0,
        2.0, -2.0, 3.0, -1.0,
        -3.0, 1.0, -4.0, 0.0,
        3.0, -1.0, 2.0, -2.0,
    ) / 255.0;

    let noise = vec3f(dither[coords.x % 4][coords.y % 4]);
    let dithered = clamp(rgba.value + vec4f(noise, 0.0), vec4f(0.0), vec4f(1.0));
    return RgbaNorm(dithered);
}

fn rgba_norm_to_rgb5m(rgba: RgbaNorm) -> Rgb5m {
    var r = unorm_to_u5(rgba.value.r);
    var g = unorm_to_u5(rgba.value.g);
    var b = unorm_to_u5(rgba.value.b);
    return Rgb5m(r | (g << 5) | (b << 10));
}

// A 32-bit RGBA color.
struct Rgba8 {
    value: vec4u,
}

fn rgba8_normalize(rgba: Rgba8) -> RgbaNorm {
    return RgbaNorm(vec4f(rgba.value) / 255.0);
}

