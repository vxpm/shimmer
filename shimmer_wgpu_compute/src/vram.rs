use crate::context::Context;
use shimmer_core::gpu::renderer::CopyToVram;
use std::sync::Arc;
use zerocopy::IntoBytes;

pub const VRAM_WIDTH: u16 = 1024;
pub const VRAM_HEIGHT: u16 = 512;

pub struct Vram {
    ctx: Arc<Context>,

    buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: Arc<wgpu::BindGroup>,
}

impl Vram {
    pub fn new(ctx: Arc<Context>) -> Self {
        let buffer = ctx.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("vram"),
            size: 1024 * 512 * 8,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout =
            ctx.device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("vram"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let bind_group = ctx.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vram"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        Self {
            ctx,

            buffer,
            bind_group_layout,
            bind_group: Arc::new(bind_group),
        }
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn bind_group(&self) -> &Arc<wgpu::BindGroup> {
        &self.bind_group
    }

    pub fn copy_to_vram(&mut self, copy: CopyToVram) {
        let mut row_padded = Vec::new();
        for (row_index, row) in copy
            .data
            .chunks(copy.width.value() as usize * 2)
            .enumerate()
        {
            row_padded.clear();
            row_padded.extend(row.iter().map(|v| *v as u32));

            let row_start = (copy.y.value() as usize + row_index) * (VRAM_WIDTH as usize) * 8
                + copy.x.value() as usize * 8;

            self.ctx
                .queue()
                .write_buffer(&self.buffer, row_start as u64, row_padded.as_bytes());
        }

        self.ctx.queue().submit([]);
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
}
