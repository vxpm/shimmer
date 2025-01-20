//!include utils

const TEXPAGE_LEN: u32 = 256;

const TEXKIND_NONE: u32 = 0;
const TEXKIND_LUT4: u32 = 1;
const TEXKIND_LUT8: u32 = 2;
const TEXKIND_FULL: u32 = 3;

const SHADING_FLAT: u32 = 0;
const SHADING_GOURAUD: u32 = 1;

struct VertexIn {
    @builtin(vertex_index) index: u32,
    @location(0) rgba: vec4<u32>,
    @location(1) xy: vec2<i32>,
    @location(2) uv: vec2<u32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) rgba: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) config_index: u32,
};

struct Config {
    texkind: u32,
    shading: u32,
    clut_x: u32,
    clut_y: u32,
    texpage_x: u32,
    texpage_y: u32,
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;

    var pos = vec2<f32>(
        unorm_to_snorm(f32(in.xy.x) / 1024.0),
        -unorm_to_snorm(f32(in.xy.y) / 512.0)
    );
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.rgba = vec4<f32>(in.rgba) / 255.0;
    out.uv = vec2<f32>(in.uv) / 255.0;
    out.config_index = in.index / 3;

    return out;
}

@group(0) @binding(0) var vram: texture_2d<u32>;
@group(1) @binding(0) var<storage, read> configs: array<Config>;

const dither: mat4x4f = mat4x4f(
    -4.0, 0.0, -3.0, 1.0,
    2.0, -2.0, 3.0, -1.0,
    -3.0, 1.0, -4.0, 0.0,
    3.0, -1.0, 2.0, -2.0,
) / 255.0;

fn dither_norm_rgba(coords: vec2u, value: vec4f) -> vec4f {
    var noise = vec3f(dither[coords.x % 4][coords.y % 4]);
    var dithered = clamp(value + vec4f(noise, 0.0), vec4f(0.0), vec4f(1.0));
    return dithered;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) u32 {
    var config = configs[in.config_index];

    var vram_coords = vec2u(u32(in.position.x), u32(in.position.y));
    var clut_coords = vec2u(config.clut_x, config.clut_y);
    var texpage_base_coords = vec2u(config.texpage_x, config.texpage_y);

    var rel_texpage_coords_f32 = vec2f(in.uv.x, in.uv.y) * f32(TEXPAGE_LEN - 1u);
    var rel_texpage_coords = vec2u(round(rel_texpage_coords_f32));

    var color: u32;
    switch config.texkind {
        case TEXKIND_LUT4 {
            var texel_coords = texpage_base_coords + rel_texpage_coords / vec2u(4, 1);
            var texel = textureLoad(vram, texel_coords, 0).r;

            var clut_index = extractBits(texel, 4 * (vram_coords.x % 4), 4u);
            var clut_color = textureLoad(vram, clut_coords + vec2u(clut_index, 0), 0).r;

            if clut_color == RGB5M_TRANSPARENT {
                color = textureLoad(vram, vram_coords, 0).r;
            } else {
                color = clut_color;
            }
        }
        case TEXKIND_LUT8 {
            var texel_coords = texpage_base_coords + rel_texpage_coords / vec2u(2, 1);
            var texel = textureLoad(vram, texel_coords, 0).r;

            var clut_index = extractBits(texel, 8 * (vram_coords.x % 2), 8u);
            var clut_color = textureLoad(vram, clut_coords + vec2u(clut_index, 0), 0).r;

            if clut_color == RGB5M_TRANSPARENT {
                color = textureLoad(vram, vram_coords, 0).r;
            } else {
                color = clut_color;
            }
        }
        case TEXKIND_FULL {
            var texel_coords = texpage_base_coords + rel_texpage_coords;
            var texel = textureLoad(vram, texel_coords, 0).r;

            if texel == RGB5M_TRANSPARENT {
                color = textureLoad(vram, vram_coords, 0).r;
            } else {
                color = texel;
            }
        }
        case TEXKIND_NONE {
            if config.shading == SHADING_GOURAUD {
                color = unorm_rgba_to_rgb5m(dither_norm_rgba(vram_coords, in.rgba));
            } else {
                color = unorm_rgba_to_rgb5m(in.rgba);
            }
        }
        default: {
            // shouldn't ever happen!
            color = RGB5M_PLACEHOLDER;
        }
    }

    return color;
}
