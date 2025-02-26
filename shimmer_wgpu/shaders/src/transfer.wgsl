//!include vram

struct Config {
    source: vec2u,
    destination: vec2u,
    dimensions: vec2u,
    check_mask: u32,
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
    for (var y: u32 = config.source.y; y < config.source.y + config.dimensions.y; y += 1u) {
        for (var x: u32 = config.source.x; x < config.source.x + config.dimensions.x; x += 1u) {
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
    for (var y: u32 = config.source.y; y < config.source.y + config.dimensions.y; y += 1u) {
        for (var x: u32 = config.source.x; x < config.source.x + config.dimensions.x; x += 1u) {
            let vram_index = y * VRAM_WIDTH + x;

            if (config.check_mask > 0) && ((vram[2 * vram_index + 1] & 0x80) > 0) {
                i += 2u;
                continue;
            }

            vram[2 * vram_index] = buffer[i];
            vram[2 * vram_index + 1] = buffer[i + 1];

            i += 2u;
        }
    }
}

@compute @workgroup_size(1, 1, 1)
fn transfer_from_vram_to_vram(@builtin(global_invocation_id) global_id: vec3u) {
    var i = 0u;
    for (var offset_y: u32 = 0; offset_y < config.dimensions.y; offset_y += 1u) {
        for (var offset_x: u32 = 0; offset_x < config.dimensions.x; offset_x += 1u) {
            let source_vram_index = (config.source.y + offset_y) * VRAM_WIDTH + (config.source.x + offset_x);
            let destination_vram_index = (config.destination.y + offset_y) * VRAM_WIDTH + (config.destination.x + offset_x);

            if (config.check_mask > 0) && ((vram[2 * destination_vram_index + 1] & 0x80) > 0) {
                i += 2u;
                continue;
            }

            vram[2 * destination_vram_index] = vram[2 * source_vram_index];
            vram[2 * destination_vram_index + 1] = vram[2 * source_vram_index + 1];

            i += 2u;
        }
    }
}
