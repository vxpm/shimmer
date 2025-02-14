use crate::context::Context;
use std::sync::Arc;

pub const VRAM_WIDTH: u16 = 1024;
pub const VRAM_HEIGHT: u16 = 512;

pub struct Vram {
    ctx: Arc<Context>,

    back_buffer: wgpu::Buffer,

    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl Vram {
    pub fn new(ctx: Arc<Context>) -> Self {
        let back_buffer = ctx.device().create_buffer(&wgpu::BufferDescriptor {
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
                    buffer: &back_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        Self {
            ctx,

            back_buffer,

            bind_group_layout,
            bind_group,
        }
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
