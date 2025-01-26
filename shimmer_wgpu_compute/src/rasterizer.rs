use crate::{context::Context, vram::Vram};
use shimmer_core::gpu::renderer::Triangle;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use zerocopy::{FromBytes, Immutable, IntoBytes};

#[repr(C, align(16))]
#[derive(FromBytes, IntoBytes, Immutable)]
struct SimpleVertex {
    color_r: u32,
    color_g: u32,
    color_b: u32,
    color_a: u32,
    x: u32,
    y: u32,
    _padding: [u32; 2],
}

#[repr(C)]
#[derive(FromBytes, IntoBytes, Immutable)]
struct TrianglePrimitive {
    vertices: [SimpleVertex; 3],
}

pub struct Rasterizer {
    ctx: Arc<Context>,

    vram_bind_group: Arc<wgpu::BindGroup>,
    triangles_bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,

    triangles: Vec<TrianglePrimitive>,
}

impl Rasterizer {
    pub fn new(ctx: Arc<Context>, vram: &Vram) -> Self {
        let shader = ctx
            .device()
            .create_shader_module(wgpu::include_wgsl!("../shaders/built/render.wgsl"));

        let triangles_bind_group_layout =
            ctx.device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("rasterizer triangles"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let pipeline_layout =
            ctx.device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[vram.bind_group_layout(), &triangles_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = ctx
            .device()
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("rasterizer"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("render"),
                compilation_options: Default::default(),
                cache: None,
            });

        Self {
            ctx,

            vram_bind_group: vram.bind_group().clone(),
            triangles_bind_group_layout,
            pipeline,

            triangles: Vec::with_capacity(128),
        }
    }

    pub fn enqueue(&mut self, triangle: Triangle) {
        self.triangles.push(TrianglePrimitive {
            vertices: triangle.vertices.map(|v| SimpleVertex {
                color_r: 255,
                color_g: 255,
                color_b: 255,
                color_a: 255,
                x: v.x.value() as u32,
                y: v.y.value() as u32,
                _padding: [0; 2],
            }),
        });
    }

    pub fn flush(&mut self) {
        if self.triangles.is_empty() {
            return;
        }

        println!("rendering {} triangles", self.triangles.len());

        // build triangles buffer
        let triangles_buffer =
            self.ctx
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("triangles"),
                    usage: wgpu::BufferUsages::STORAGE,
                    contents: self.triangles.as_bytes(),
                });

        let triangle_bind_group = self
            .ctx
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("triangles"),
                layout: &self.triangles_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &triangles_buffer,
                        offset: 0,
                        size: None,
                    }),
                }],
            });

        let mut encoder = self
            .ctx
            .device()
            .create_command_encoder(&Default::default());

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("rasterizer"),
            timestamp_writes: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &*self.vram_bind_group, &[]);
        pass.set_bind_group(1, &triangle_bind_group, &[]);
        pass.dispatch_workgroups(1024 / 8, 512 / 8, 1);

        std::mem::drop(pass);
        self.ctx.queue().submit([encoder.finish()]);
        self.ctx.device().poll(wgpu::Maintain::Wait);

        self.triangles.clear();
    }
}
