//!include vram

struct Config {
    position: vec2u,
    dimensions: vec2u,
}

@group(0) @binding(0)
var<storage, read_write> vram: array<u32>;
@group(1) @binding(0)
var<storage, read> config: Config;
@group(1) @binding(1)
var<storage, read_write> buffer: array<u32>;

@compute @workgroup_size(1, 1, 1)
fn transfer_from_vram_to_buffer(@builtin(global_invocation_id) global_id: vec3u) {
    var i = 0u;
    for (var y: u32 = config.position.y; y < config.position.y + config.dimensions.y; y += 1u) {
        for (var x: u32 = config.position.x; x < config.position.x + config.dimensions.x; x += 1u) {
            let vram_index = y * VRAM_WIDTH + x;
            buffer[i] = vram[2 * vram_index];
            buffer[i + 1] = vram[2 * vram_index + 1];

            i += 2u;
        }
    }
}

@compute @workgroup_size(1, 1, 1)
fn transfer_from_buffer_to_vram(@builtin(global_invocation_id) global_id: vec3u) {
    var i = 0u;
    for (var y: u32 = config.position.y; y < config.position.y + config.dimensions.y; y += 1u) {
        for (var x: u32 = config.position.x; x < config.position.x + config.dimensions.x; x += 1u) {
            let vram_index = y * VRAM_WIDTH + x;
            vram[2 * vram_index] = buffer[i];
            vram[2 * vram_index + 1] = buffer[i + 1];

            i += 2u;
        }
    }
}
