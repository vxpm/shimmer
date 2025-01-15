struct VertexIn {
    @location(0) rgba: vec4<u32>,
    @location(1) xy: vec2<i32>,
    @location(2) uv: vec2<u32>,
};

struct VertexOut {
    @location(0) color: vec4<f32>,
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;

    out.color = vec4<f32>(in.rgba);
    out.clip_position = vec4<f32>(f32(in.xy.x) / 1024.0, f32(in.xy.y) / 512.0, 0.0, 1.0);
    out.clip_position = 2 * out.clip_position - vec4<f32>(1.0, 1.0, 0.0, 0.0);

    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return in.color;
}
