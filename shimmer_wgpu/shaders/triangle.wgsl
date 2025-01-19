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

const TEXKIND_LUT4: u32 = 0;
const TEXKIND_LUT8: u32 = 1;
const TEXKIND_FULL: u32 = 2;
const TEXKIND_NONE: u32 = 3;

const SHADING_FLAT: u32 = 0;
const SHADING_GOURAUD: u32 = 1;

struct Config {
    texkind: u32,
    clut_x: u32,
    clut_y: u32,
    texpage_x: u32,
    texpage_y: u32,
}

fn to_signed_range(value: f32) -> f32 {
    return 2.0 * value - 1.0;
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

@group(0) @binding(0) var vram: texture_2d<u32>;
@group(1) @binding(0) var<uniform> config: Config;

const TEXPAGE_WIDTH: u32 = 256;
const TEXPAGE_HEIGHT: u32 = 256;

const dither: mat4x4f = mat4x4f(
    -4.0, 0.0, -3.0, 1.0,
    2.0, -2.0, 3.0, -1.0,
    -3.0, 1.0, -4.0, 0.0,
    3.0, -1.0, 2.0, -2.0,
) / 255.0;

fn norm_to_u5(value: f32) -> u32 {
    return u32(value * 31.0) & 0x1Fu;
}

fn norm_rgba_to_rgb5m(rgba: vec4<f32>) -> u32 {
    var r = norm_to_u5(rgba.r);
    var g = norm_to_u5(rgba.g);
    var b = norm_to_u5(rgba.b);
    return r | (g << 5) | (b << 10);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) u32 {
    var texpage_base = vec2u(config.texpage_x, config.texpage_y);

    // calculate coords of data in texpage
    var texpage_coords_f32 = vec2f(in.uv.x, in.uv.y) * 255.0;
    var texpage_coords = vec2u(round(texpage_coords_f32));

    var color = 0xDEADu;
    switch config.texkind {
        case TEXKIND_LUT4 {
            texpage_coords.x /= 4u;

            // convert from page coords to vram coords
            var vram_coords = texpage_base + texpage_coords;
            var index = u32(in.position.x) % 4;

            // get data in vram
            var data = textureLoad(vram, vram_coords, 0).r;

            // get clut index
            var clut_index = extractBits(data, 4 * index, 4u);

            // sample pallete
            var clut_color = textureLoad(vram, vec2u(config.clut_x + clut_index, config.clut_y), 0).r;
            if clut_color == 0 {
                color = textureLoad(
                    vram,
                    vec2u(
                        u32(in.position.x),
                        u32(in.position.y),
                    ),
                    0
                ).r;
            } else {
                color = clut_color;
            }
        }
        case TEXKIND_LUT8 {
            texpage_coords.x /= 2u;

            // convert from page coords to vram coords
            var vram_coords = texpage_base + texpage_coords;
            var index = u32(in.position.x) % 4;

            // get data in vram
            var data = textureLoad(vram, vram_coords, 0).r;

            // get clut index
            var clut_index = extractBits(data, 8 * index, 8u);

            // sample pallete
            var clut_color = textureLoad(vram, vec2u(config.clut_x + clut_index, config.clut_y), 0).r;
            if clut_color == 0 {
                color = textureLoad(
                    vram,
                    vec2u(
                        u32(in.position.x),
                        u32(in.position.y),
                    ),
                    0
                ).r;
            } else {
                color = clut_color;
            }
        }
        case TEXKIND_FULL {
            // convert from page coords to vram coords
            var vram_coords = texpage_base + texpage_coords;
            var index = u32(in.position.x) % 4;

            // sample color in vram
            var data = textureLoad(vram, vram_coords, 0).r;

            if data == 0 {
                color = textureLoad(
                    vram,
                    vec2u(
                        u32(in.position.x),
                        u32(in.position.y),
                    ),
                    0
                ).r;
            } else {
                color = data;
            }
        }
        case TEXKIND_NONE {
            var x = u32(in.position.x);
            var y = u32(in.position.y);
            var noise = vec3f(dither[x % 4][y % 4]);
            var result = clamp(in.rgba + vec4f(noise, 0.0), vec4f(0.0), vec4f(1.0));
            color = norm_rgba_to_rgb5m(result);
        }
        default: {
            color = 0xBABEu;
        }
    }

    return color;
}
