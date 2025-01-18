struct VertexIn {
    @location(0) rgba: vec4<u32>,
    @location(1) xy: vec2<i32>,
    @location(2) uv: vec2<u32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) rgba: vec4<f32>,
    @location(1) uv: vec2<f32>,
};

fn to_signed_range(value: f32) -> f32 {
    return 2.0 * value - 1.0;
}

fn to_unsigned_range(value: f32) -> f32 {
    return (value + 1.0) / 2.0;
}

fn norm_to_u5(value: f32) -> u32 {
    return u32(value * 31.0) & 0x1Fu;
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
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.rgba = vec4<f32>(in.rgba) / 255.0;
    out.uv = vec2<f32>(in.uv) / 255.0;

    return out;
}

const KIND_LUT4: u32 = 0;
const KIND_LUT8: u32 = 1;
const KIND_FULL: u32 = 2;
const KIND_NONE: u32 = 3;

struct Extra {
    kind: u32,
    clut_x: u32,
    clut_y: u32,
    texpage_x: u32,
    texpage_y: u32,
}

@group(0) @binding(0) var vram: texture_2d<u32>;
@group(1) @binding(0) var<uniform> extra: Extra;

const TEXPAGE_WIDTH: u32 = 256;
const TEXPAGE_HEIGHT: u32 = 256;

@fragment
fn fs_main(in: VertexOut) -> @location(0) u32 {
    // calculate texpage base coords
    var texpage_base = vec2u(
        extra.texpage_x * 64,
        extra.texpage_y * 256,
    );

    // calculate coords of data in texpage
    var texpage_coords_f32 = vec2f(in.uv.x, in.uv.y) * 255.0;
    var texpage_coords = vec2u(round(texpage_coords_f32));

    var color: u32 = 0x0000;
    switch extra.kind {
        case KIND_LUT4 {
            texpage_coords.x /= 4u;

            // convert from page coords to vram coords
            var vram_coords = texpage_base + texpage_coords;
            var index = u32(in.position.x) % 4;

            // get data in vram
            var data = textureLoad(vram, vram_coords, 0).r;

            // get pallete index
            var pallete_index = extractBits(data, 4 * index, 4u);

            // sample pallete
            var pallete_color = textureLoad(vram, vec2u(extra.clut_x + pallete_index, extra.clut_y), 0).r;
            if pallete_color == 0 {
                color = textureLoad(
                    vram,
                    vec2u(
                        u32(in.position.x),
                        u32(in.position.y),
                    ),
                    0
                ).r;
            } else {
                color = pallete_color;
            }
        }
        case KIND_LUT8 {
            color = 0xDEADu;
        }
        case KIND_FULL {
            color = 0xDEADu;
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
