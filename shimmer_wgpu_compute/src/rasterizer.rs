mod dirty;

use crate::{
    context::Context,
    util::ShaderSlice,
    vram::{VRAM_HEIGHT, VRAM_WIDTH, Vram},
};
use dirty::{DirtyRegions, Region};
use encase::{ShaderType, StorageBuffer};
use glam::{IVec2, UVec2, UVec4};
use shimmer_core::gpu::{
    cmd::environment::TexPageDepth,
    renderer::{Rectangle as RendererRectangle, Triangle as RendererTriangle},
};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use zerocopy::{Immutable, IntoBytes};

#[derive(Debug, Clone, ShaderType)]
struct Vertex {
    coords: IVec2,
    rgba: UVec4,
    uv: UVec2,
}

impl Vertex {
    /// Sorts an slice of vertices in counter-clockwise order.
    pub fn sort(vertices: &mut [Self]) {
        let center =
            vertices.iter().fold(IVec2::ZERO, |acc, v| acc + v.coords) / (vertices.len() as i32);

        vertices.sort_by_key(|v| {
            let relative = v.coords - center;
            let x = relative.x as f32;
            let y = relative.y as f32;

            ordered_float::OrderedFloat(y.atan2(x))
        });
    }
}

#[derive(Debug, Clone, ShaderType, Default)]
struct TextureConfig {
    mode: u32,
    clut: UVec2,
    texpage: UVec2,
}

#[derive(Debug, Clone, ShaderType)]
struct Triangle {
    vertices: [Vertex; 3],
    shading_mode: u32,
    texture_config: TextureConfig,
}

impl Triangle {
    pub fn bounding_region(&self) -> Region {
        let mut min_x = u16::MAX;
        let mut max_x = u16::MIN;
        let mut min_y = u16::MAX;
        let mut max_y = u16::MIN;

        for vertex in &self.vertices {
            let coords = vertex.coords;
            min_x = min_x.min(coords.x.clamp(0, VRAM_WIDTH as i32) as u16);
            max_x = max_x.max(coords.x.clamp(0, VRAM_WIDTH as i32) as u16);

            min_y = min_y.min(coords.y.clamp(0, VRAM_HEIGHT as i32) as u16);
            max_y = max_y.max(coords.y.clamp(0, VRAM_HEIGHT as i32) as u16);
        }

        Region::from_extremes((min_x, min_y), (max_x, max_y))
    }
}

#[derive(Debug, Clone, ShaderType)]
struct Rectangle {
    top_left: IVec2,
    top_left_uv: UVec2,
    dimensions: UVec2,
    rgba: UVec4,
    texture_config: TextureConfig,
}

impl Rectangle {
    pub fn bounding_region(&self) -> Region {
        Region::new(
            (
                self.top_left.x.clamp(0, VRAM_WIDTH as i32) as u16,
                self.top_left.y.clamp(0, VRAM_WIDTH as i32) as u16,
            ),
            (self.dimensions.x as u16, self.dimensions.y as u16),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoBytes, Immutable)]
#[repr(u32)]
enum Command {
    Barrier,
    Triangle,
    Rectangle,
}

pub struct Rasterizer {
    ctx: Arc<Context>,

    vram_bind_group: Arc<wgpu::BindGroup>,
    rasterizer_bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,

    commands: Vec<Command>,
    triangles: Vec<Triangle>,
    rectangles: Vec<Rectangle>,
    dirty: DirtyRegions,
}

impl Rasterizer {
    pub fn new(ctx: Arc<Context>, vram: &Vram) -> Self {
        let shader = ctx
            .device()
            .create_shader_module(wgpu::include_wgsl!("../shaders/built/rasterizer.wgsl"));

        let rasterizer_bind_group_layout =
            ctx.device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("rasterizer data"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let pipeline_layout =
            ctx.device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[vram.bind_group_layout(), &rasterizer_bind_group_layout],
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

            vram_bind_group: vram.back_bind_group().clone(),
            rasterizer_bind_group_layout,
            pipeline,

            commands: Vec::with_capacity(64),
            triangles: Vec::with_capacity(64),
            rectangles: Vec::with_capacity(64),
            dirty: DirtyRegions::default(),
        }
    }

    pub fn enqueue_triangle(&mut self, triangle: RendererTriangle) {
        let texture_config = if let Some(texture) = &triangle.texture {
            let region = Region::new(
                (
                    u16::from(texture.texpage.x_base().value()) * 64,
                    u16::from(texture.texpage.y_base().value()) * 256,
                ),
                (64, 256),
            );

            if self.dirty.is_dirty(region) {
                self.commands.push(Command::Barrier);
                self.dirty.clear();
            }

            TextureConfig {
                mode: match texture.texpage.depth() {
                    TexPageDepth::Nibble => 1,
                    TexPageDepth::Byte => 2,
                    TexPageDepth::Full | TexPageDepth::Reserved => 3,
                },
                clut: UVec2::new(
                    texture.clut.x_by_16().value() as u32 * 16,
                    texture.clut.y().value() as u32,
                ),
                texpage: UVec2::new(
                    texture.texpage.x_base().value() as u32 * 64,
                    texture.texpage.y_base().value() as u32 * 256,
                ),
            }
        } else {
            TextureConfig::default()
        };

        let mut primitive = Triangle {
            vertices: triangle.vertices.map(|v| Vertex {
                coords: IVec2::new(v.x.value() as i32, v.y.value() as i32),
                rgba: UVec4::new(v.color.r as u32, v.color.g as u32, v.color.b as u32, 255),
                uv: UVec2::new(v.u as u32, v.v as u32),
            }),
            shading_mode: triangle.shading as u32,
            texture_config,
        };

        Vertex::sort(&mut primitive.vertices);
        self.dirty.mark(primitive.bounding_region());
        self.commands.push(Command::Triangle);
        self.triangles.push(primitive);
    }

    pub fn enqueue_rectangle(&mut self, rectangle: RendererRectangle) {
        let texture_config = if let Some(texture) = &rectangle.texture {
            let region = Region::new(
                (
                    u16::from(texture.texpage.x_base().value()) * 64,
                    u16::from(texture.texpage.y_base().value()) * 256,
                ),
                (64, 256),
            );

            if self.dirty.is_dirty(region) {
                self.commands.push(Command::Barrier);
                self.dirty.clear();
            }

            TextureConfig {
                mode: match texture.texpage.depth() {
                    TexPageDepth::Nibble => 1,
                    TexPageDepth::Byte => 2,
                    TexPageDepth::Full | TexPageDepth::Reserved => 3,
                },
                clut: UVec2::new(
                    texture.clut.x_by_16().value() as u32 * 16,
                    texture.clut.y().value() as u32,
                ),
                texpage: UVec2::new(
                    texture.texpage.x_base().value() as u32 * 64,
                    texture.texpage.y_base().value() as u32 * 256,
                ),
            }
        } else {
            TextureConfig::default()
        };

        let primitive = Rectangle {
            top_left: IVec2::new(rectangle.x.value() as i32, rectangle.y.value() as i32),
            top_left_uv: UVec2::new(rectangle.u as u32, rectangle.v as u32),
            dimensions: UVec2::new(
                rectangle.width.value() as u32,
                rectangle.height.value() as u32,
            ),
            rgba: UVec4::new(
                rectangle.color.r as u32,
                rectangle.color.g as u32,
                rectangle.color.b as u32,
                255,
            ),
            texture_config,
        };

        self.dirty.mark(primitive.bounding_region());
        self.commands.push(Command::Rectangle);
        self.rectangles.push(primitive);
    }

    pub fn flush(&mut self) {
        if self.commands.is_empty() {
            return;
        }

        // commands buffer
        let commands_buffer =
            self.ctx
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("commands"),
                    usage: wgpu::BufferUsages::STORAGE,
                    contents: &self.commands.as_bytes(),
                });

        // primitives
        let mut data = StorageBuffer::new(Vec::new());
        let triangles = ShaderSlice::new(&self.triangles);
        data.write(&triangles).unwrap();

        let triangles_buffer =
            self.ctx
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("triangles"),
                    usage: wgpu::BufferUsages::STORAGE,
                    contents: &data.into_inner(),
                });

        let mut data = StorageBuffer::new(Vec::new());
        let rectangles = ShaderSlice::new(&self.rectangles);
        data.write(&rectangles).unwrap();

        let rectangles_buffer =
            self.ctx
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("rectangles"),
                    usage: wgpu::BufferUsages::STORAGE,
                    contents: &data.into_inner(),
                });

        // bind group
        let rasterizer_bind_group =
            self.ctx
                .device()
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("rasterizer data"),
                    layout: &self.rasterizer_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: &commands_buffer,
                                offset: 0,
                                size: None,
                            }),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: &triangles_buffer,
                                offset: 0,
                                size: None,
                            }),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: &rectangles_buffer,
                                offset: 0,
                                size: None,
                            }),
                        },
                    ],
                });

        // render
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
        pass.set_bind_group(1, &rasterizer_bind_group, &[]);
        pass.dispatch_workgroups(1024 / 8, 512 / 8, 1);

        std::mem::drop(pass);
        self.ctx.queue().submit([encoder.finish()]);
        self.ctx.device().poll(wgpu::Maintain::Wait);

        self.commands.clear();
        self.triangles.clear();
        self.dirty.clear();
    }
}
