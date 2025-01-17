struct VertexIn {
    @location(0) rgba: vec4<u32>,
    @location(1) xy: vec2<i32>,
    @location(2) uv: vec2<u32>,
};

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) rgba: vec4<f32>,
    @location(1) uv: vec2<f32>,
};

fn to_signed_range(value: f32) -> f32 {
    return 2.0 * value - 1.0;
}

fn norm_to_u5(value: f32) -> u32 {
    return u32(value * 32.0) & 0x1F;
}

fn rgba8_to_rgb5m(rgba: vec4<f32>) -> u32 {
    var r = norm_to_u5(rgba.r);
    var g = norm_to_u5(rgba.g);
    var b = norm_to_u5(rgba.b);
    return r | (g << 5) | (b << 10);
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;

    var pos = vec2<f32>(
        to_signed_range(f32(in.xy.x) / 1024.0),
        -to_signed_range(f32(in.xy.y) / 512.0)
    );
    out.clip_position = vec4<f32>(pos, 0.0, 1.0);
    out.rgba = vec4<f32>(in.rgba) / 255.0;
    out.uv = vec2<f32>(in.uv);

    return out;
}

const KIND_LUT4: u32 = 0;
const KIND_LUT8: u32 = 1;
const KIND_FULL: u32 = 2;
const KIND_NONE: u32 = 3;

struct TexturedInfo {
    kind: u32,
    clut: vec2<u32>,
    texpage: vec2<u32>,
}

@group(0) @binding(0) var tex_view: texture_2d<u32>;
@group(1) @binding(0) var<uniform> textured_info: TexturedInfo;

const TEXPAGE_WIDTH = 256.0;
const TEXPAGE_HEIGHT = 256.0;

@fragment
fn fs_main(in: VertexOut) -> @location(0) u32 {
    // calculate texpage base coords
    var texpage_base = vec2u(
        textured_info.texpage.x * 64,
        textured_info.texpage.y * 256,
    );

    // calculate offset into texpage
    var texpage_offset = vec2u(
        u32(in.uv.x * TEXPAGE_WIDTH),
        u32(in.uv.y * TEXPAGE_HEIGHT)
    );

    var color: u32 = 0xDEAD;
    switch textured_info.kind {
        case KIND_LUT4 {
            var rgba = vec4<f32>(1.0, 1.0, 1.0, 1.0);
            color = rgba8_to_rgb5m(rgba);
        }
        case KIND_LUT8 {
            color = 0xFF00u;
        }
        case KIND_FULL {
            color = 0xFF0000u;
        }
        case KIND_NONE {
            color = rgba8_to_rgb5m(in.rgba);
        }
        default: {
            color = 0xFFFFFFu;
        }
    }

    return color;
}
