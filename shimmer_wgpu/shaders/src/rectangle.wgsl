//!include consts
//!include utils

struct Config {
    @builtin(vertex_index) index: u32,
    @location(0) rgba: vec4u,
    @location(1) xy: vec2i,
    @location(2) dimensions: vec2u,
    @location(3) texkind: u32,
    @location(4) clut: vec2u,
    @location(5) texpage: vec2u,
    @location(6) uv: vec2u,
}

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) top_left: vec2i,
    @location(1) rgba: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) texkind: u32,
    @location(4) clut: vec2u,
    @location(5) texpage: vec2u,
};

const vertex_offset: array<vec2f, 4> = array<vec2f, 4>(
    vec2f(-1.0, 1.0),
    vec2f(-1.0, -1.0),
    vec2f(1.0, 1.0),
    vec2f(1.0, -1.0),
);

@vertex
fn vs_main(config: Config) -> VertexOut {
    var out: VertexOut;

    var top_left = vec2f(f32(config.xy.x), f32(config.xy.y));
    var vertex_pos = top_left + snorm_to_unorm_vec2(vertex_offset[config.index]) * vec2f(config.dimensions);
    var snorm_pos = unorm_to_snorm_vec2(vertex_pos / vec2f(1024.0, 512.0));

    out.position = vec4f(
        snorm_pos.x,
        -snorm_pos.y,
        0.0,
        1.0
    );
    out.top_left = config.xy;
    out.rgba = vec4f(config.rgba) / 255.0;
    out.uv = vec2f(config.uv) / 255.0;

    out.texkind = config.texkind;
    out.clut = config.clut;
    out.texpage = config.texpage;

    return out;
}

@group(0) @binding(0) var vram: texture_2d<u32>;

@fragment
fn fs_main(in: VertexOut) -> @location(0) u32 {
    var vram_coords = vec2u(u32(in.position.x), u32(in.position.y));
    var clut_coords = vec2u(in.clut.x, in.clut.y);
    var texpage_base_coords = vec2u(in.texpage.x, in.texpage.y);

    var coords_offset = vec2u(vram_coords.x - u32(in.top_left.x), vram_coords.y - u32(in.top_left.y));
    var rel_texpage_coords_f32 = vec2f(in.uv.x, in.uv.y) * f32(TEXPAGE_LEN - 1u);
    var rel_texpage_coords = vec2u(round(rel_texpage_coords_f32)) + coords_offset;

    var color: u32;
    switch in.texkind {
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
            color = unorm_rgba_to_rgb5m(in.rgba);
        }
        default: {
            // shouldn't ever happen!
            color = RGB5M_PLACEHOLDER;
        }
    }

    return color;
}
