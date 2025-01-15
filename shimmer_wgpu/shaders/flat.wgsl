struct VertexIn {
    @location(0) rgba: vec4<u32>,
    @location(1) xy: vec2<i32>,
    @location(2) uv: vec2<u32>,
};

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

fn to_signed_range(value: f32) -> f32 {
    return 2.0 * value - 1.0;
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;

    out.color = vec4<f32>(in.rgba) / 255.0;

    var pos = vec2<f32>(
        to_signed_range(f32(in.xy.x) / 1024.0),
        to_signed_range(f32(in.xy.y) / 512.0)
    );
    out.clip_position = vec4<f32>(pos, 0.0, 1.0);

    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return in.color;
}
