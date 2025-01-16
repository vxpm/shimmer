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
var tex: texture_2d<f32>;
@group(0) @binding(1)
var tex_sampler: sampler;

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    var color = textureSample(tex, tex_sampler, in.uv);
    if color.a == 0 {
        color = vec4<f32>(0.2, 0.0, 0.0, 1.0);
    }

    return color;
}
