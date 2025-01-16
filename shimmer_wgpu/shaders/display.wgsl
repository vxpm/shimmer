struct VertexOut {
    @location(0) uv: vec2<f32>,
    @builtin(position) clip_position: vec4<f32>,
}

var<private> vertex_positions: array<vec2<f32>, 4> = array<vec2<f32>, 4>(
    vec2<f32>(-1.0, 1.0),
    vec2<f32>(-1.0, -1.0),
    vec2<f32>(1.0, 1.0),
    vec2<f32>(1.0, -1.0),
);

var<private> vertex_uvs: array<vec2<f32>, 4> = array<vec2<f32>, 4>(
    vec2<f32>(0.0, 1.0),
    vec2<f32>(0.0, 0.0),
    vec2<f32>(1.0, 1.0),
    vec2<f32>(1.0, 0.0),
);

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOut {
    var out: VertexOut;

    out.uv = vertex_uvs[index];
    out.clip_position = vec4<f32>(vertex_positions[index], 0.0, 1.0);

    return out;
}

@group(0) @binding(0)
var tex: texture_2d<u32>;
@group(0) @binding(1)
var tex_sampler: sampler;

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    var x = in.uv.x * 512.0;
    var y = in.uv.y * 256.0;

    // assume 16 bit (rgb555a1) mode
    var data = textureLoad(tex, vec2<u32>(u32(x), u32(y)), 0);
    var r = data.r & 0x1Fu;
    var g = (data.g & 0x1Fu);
    var b = (data.b & 0x1Fu);
    var color = vec4<f32>(
        f32(r) / 32.0,
        f32(g) / 32.0,
        f32(b) / 32.0,
        1.0,
    );

    return color;
}
