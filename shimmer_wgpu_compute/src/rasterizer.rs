use crate::{context::Context, vram::Vram};
use encase::{ShaderType, StorageBuffer};
use glam::{IVec2, UVec4};
use shimmer_core::gpu::renderer::Triangle;
use std::sync::Arc;
use wgpu::util::DeviceExt;

#[derive(Debug, Clone, ShaderType)]
struct SimpleVertex {
    rgba: UVec4,
    coords: IVec2,
}

#[derive(Debug, Clone, ShaderType)]
struct TrianglePrimitive {
    vertices: [SimpleVertex; 3],
}

impl TrianglePrimitive {
    pub fn sort(&mut self) {
        let center =
            (self.vertices[0].coords + self.vertices[1].coords + self.vertices[2].coords) / 3;

        self.vertices.sort_by_key(|v| {
            let relative = v.coords - center;
            let x = relative.x as f32;
            let y = relative.y as f32;

            ordered_float::OrderedFloat(y.atan2(x))
        });
    }
}

#[derive(ShaderType)]
struct TrianglePrimitiveArray {
    #[size(runtime)]
    triangles: Vec<TrianglePrimitive>,
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
        let mut triangle = TrianglePrimitive {
            vertices: triangle.vertices.map(|v| SimpleVertex {
                rgba: UVec4::new(v.color.r as u32, v.color.g as u32, v.color.b as u32, 255),
                coords: IVec2::new(v.x.value() as i32, v.y.value() as i32),
            }),
        };

        triangle.sort();
        self.triangles.push(triangle);
    }

    pub fn flush(&mut self) {
        if self.triangles.is_empty() {
            return;
        }

        // build triangles buffer
        let mut data = StorageBuffer::new(Vec::new());
        let triangles = TrianglePrimitiveArray {
            triangles: self.triangles.clone(),
        };
        data.write(&triangles).unwrap();

        let triangles_buffer =
            self.ctx
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("triangles"),
                    usage: wgpu::BufferUsages::STORAGE,
                    contents: &data.into_inner(),
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
