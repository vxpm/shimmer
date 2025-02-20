//!include vram
//!include color

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
    vec2<f32>(0.0, 0.0),
    vec2<f32>(0.0, 1.0),
    vec2<f32>(1.0, 0.0),
    vec2<f32>(1.0, 1.0),
);

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOut {
    var out: VertexOut;

    out.uv = vertex_uvs[index];
    out.clip_position = vec4<f32>(vertex_positions[index], 0.0, 1.0);

    return out;
}

@group(0) @binding(0)
var<storage, read_write> vram: array<u32>;

struct DisplayArea {
    top_left: u32,
    dimensions: u32,
}

@group(1) @binding(0)
var<uniform> display_area: DisplayArea;

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4f {
    var top_left_x = extractBits(display_area.top_left, 0u, 16u);
    var top_left_y = extractBits(display_area.top_left, 16u, 16u);
    var dimensions_x = extractBits(display_area.dimensions, 0u, 16u);
    var dimensions_y = extractBits(display_area.dimensions, 16u, 16u);

    var x = f32(top_left_x) + in.uv.x * f32(dimensions_x);
    var y = f32(top_left_y) + in.uv.y * f32(dimensions_y);
    var vram_coords = vec2u(u32(floor(x)), u32(floor(y)));

    // assume 16 bit (rgb5m) mode
    var rgb5m = vram_get_color_rgb5m(vram_coords);
    var rgb_norm = rgb5m_to_rgb_norm(rgb5m);

    return vec4f(rgb_norm.value, 1.0);
}
