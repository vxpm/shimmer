use crate::context::Context;
use shimmer_core::gpu::renderer::CopyToVram;
use std::sync::Arc;
use zerocopy::IntoBytes;

pub const VRAM_WIDTH: u16 = 1024;
pub const VRAM_HEIGHT: u16 = 512;

pub struct Vram {
    ctx: Arc<Context>,

    back_buffer: wgpu::Buffer,
    front_buffer: wgpu::Buffer,

    bind_group_layout: wgpu::BindGroupLayout,

    back_bind_group: Arc<wgpu::BindGroup>,
    front_bind_group: Arc<wgpu::BindGroup>,
}

impl Vram {
    pub fn new(ctx: Arc<Context>) -> Self {
        let back_buffer = ctx.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("vram back"),
            size: 1024 * 512 * 8,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let front_buffer = ctx.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("vram front"),
            size: 1024 * 512 * 8,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
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

        let back_bind_group = ctx.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vram back"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &back_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        let front_bind_group = ctx.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vram front"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &front_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        Self {
            ctx,

            back_buffer,
            front_buffer,

            bind_group_layout,
            back_bind_group: Arc::new(back_bind_group),
            front_bind_group: Arc::new(front_bind_group),
        }
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn back_bind_group(&self) -> &Arc<wgpu::BindGroup> {
        &self.back_bind_group
    }

    pub fn front_bind_group(&self) -> &Arc<wgpu::BindGroup> {
        &self.front_bind_group
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

            self.ctx.queue().write_buffer(
                &self.back_buffer,
                row_start as u64,
                row_padded.as_bytes(),
            );
        }

        self.ctx.queue().submit([]);
    }

    pub fn sync(&mut self) {
        let mut encoder = self
            .ctx
            .device()
            .create_command_encoder(&Default::default());

        encoder.copy_buffer_to_buffer(&self.back_buffer, 0, &self.front_buffer, 0, 1024 * 512 * 8);
        self.ctx.queue().submit([encoder.finish()]);
    }
}
