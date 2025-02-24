//!include utils

alias BlendingMode = u32;
const BLENDING_MODE_AVG: BlendingMode = 0;
const BLENDING_MODE_ADD: BlendingMode = 1;
const BLENDING_MODE_SUB: BlendingMode = 2;
const BLENDING_MODE_ACC: BlendingMode = 3;

struct Rgb5m {
    value: u32,
}

const RGB5M_PLACEHOLDER = Rgb5m(0x7C1C);
const RGB5M_TRANSPARENT = Rgb5m(0x0000);

fn rgb5m_to_rgb_norm(rgb5m: Rgb5m) -> RgbNorm {
    var rgb5 = vec3u(
        extractBits(rgb5m.value, 0u, 5u),
        extractBits(rgb5m.value, 5u, 5u),
        extractBits(rgb5m.value, 10u, 5u)
    );

    return RgbNorm(vec3f(rgb5) / 32.0);
}

fn rgb5m_get_mask(rgb5m: Rgb5m) -> bool {
    return (rgb5m.value & 0x8000) > 0;
}

fn rgb5m_set_mask(rgb5m: Rgb5m) -> Rgb5m {
    return Rgb5m(rgb5m.value | 0x8000);
}

// A normalized RGB color.
struct RgbNorm {
    value: vec3f,
}

const RGB_NORM_PLACEHOLDER = RgbNorm(vec3f(1.0, 0.0, 0.88));

fn rgb_norm_dither(coords: vec2u, rgb: RgbNorm) -> RgbNorm {
    const dither: mat4x4f = mat4x4f(
        -4.0, 0.0, -3.0, 1.0,
        2.0, -2.0, 3.0, -1.0,
        -3.0, 1.0, -4.0, 0.0,
        3.0, -1.0, 2.0, -2.0,
    ) / 255.0;

    let noise = vec3f(dither[coords.x % 4][coords.y % 4]);
    let dithered = clamp(rgb.value + vec3f(noise), vec3f(0.0), vec3f(1.0));
    return RgbNorm(dithered);
}

fn rgb_norm_to_rgb5m(rgb: RgbNorm) -> Rgb5m {
    var r = unorm_to_u5(rgb.value.r);
    var g = unorm_to_u5(rgb.value.g);
    var b = unorm_to_u5(rgb.value.b);
    return Rgb5m(r | (g << 5) | (b << 10));
}

// A 24-bit RGB color.
struct Rgb8 {
    value: vec3u,
}

fn rgb8_to_rgb_norm(rgb: Rgb8) -> RgbNorm {
    return RgbNorm(vec3f(rgb.value) / 255.0);
}

fn rgb_norm_blend(mode: BlendingMode, bg: RgbNorm, fg: RgbNorm) -> RgbNorm {
    var blended = RGB_NORM_PLACEHOLDER.value;
    switch mode {
        case BLENDING_MODE_AVG {
            blended = (bg.value + fg.value) / 2.0;
        }
        case BLENDING_MODE_ADD {
            blended = bg.value + fg.value;
        }
        case BLENDING_MODE_SUB {
            blended = bg.value - fg.value;
        }
        case BLENDING_MODE_ACC {
            blended = bg.value + fg.value / 4.0;
        }
        default: {}
    }

    return RgbNorm(clamp(blended, vec3f(0.0), vec3f(1.0)));
}
