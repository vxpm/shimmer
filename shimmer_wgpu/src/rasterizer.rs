mod data;
mod dirty;

use crate::{context::Context, util::ShaderSlice, vram::Vram};
use data::{Config, to_buffer};
use dirty::DirtyRegions;
use glam::UVec2;
use shimmer::gpu::interface::{
    DrawingArea, Rectangle as InterfaceRectangle, Triangle as InterfaceTriangle,
};
use std::sync::Arc;
use tinylog::{info, warn};
use wgpu::util::DeviceExt;
use zerocopy::{Immutable, IntoBytes};

#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoBytes, Immutable)]
#[repr(u32)]
enum Command {
    Triangle,
    Rectangle,
}

pub struct Rasterizer {
    ctx: Arc<Context>,

    vram_bind_group: wgpu::BindGroup,

    config_bind_group_layout: wgpu::BindGroupLayout,
    data_bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,

    config: Config,
    commands: Vec<Command>,
    triangles: Vec<data::Triangle>,
    rectangles: Vec<data::Rectangle>,

    drawn_regions: DirtyRegions,
    sampled_regions: DirtyRegions,
}

impl Rasterizer {
    pub fn new(ctx: Arc<Context>, vram: &Vram) -> Self {
        let shader = ctx
            .device()
            .create_shader_module(wgpu::include_wgsl!("../shaders/built/rasterizer.wgsl"));

        let config_bind_group_layout =
            ctx.device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("rasterizer config"),
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

        let config = Config {
            drawing_area_coords: UVec2::new(0, 0),
            drawing_area_dimensions: UVec2::new(1024, 512),
        };

        let data_bind_group_layout =
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
                    bind_group_layouts: &[
                        vram.bind_group_layout(),
                        &config_bind_group_layout,
                        &data_bind_group_layout,
                    ],
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

            config,
            config_bind_group_layout,
            data_bind_group_layout,
            pipeline,

            commands: Vec::with_capacity(64),
            triangles: Vec::with_capacity(64),
            rectangles: Vec::with_capacity(64),

            drawn_regions: DirtyRegions::default(),
            sampled_regions: DirtyRegions::default(),
        }
    }

    pub fn set_drawing_area(&mut self, area: DrawingArea) {
        warn!(
            self.ctx.logger(),
            "changed drawing area"; area = area
        );

        self.config.drawing_area_coords =
            UVec2::new(u32::from(area.coords.x.value()), u32::from(area.coords.y.value()));
        self.config.drawing_area_dimensions = UVec2::new(
            u32::from(area.dimensions.width.value()),
            u32::from(area.dimensions.height.value()),
        );
    }

    pub fn enqueue_triangle(&mut self, triangle: InterfaceTriangle) {
        let triangle = data::Triangle::new(triangle);
        if let Some(sampling_region) = triangle.texconfig().sampling_region()
            && self.drawn_regions.is_dirty(sampling_region)
        {
            warn!(
                self.ctx.logger(),
                "{:?} is dirty (sampling) - flushing", sampling_region
            );
            self.flush();

            self.sampled_regions.mark(sampling_region);
        }

        let drawing_region = triangle.bounding_region();
        if self.sampled_regions.is_dirty(drawing_region) {
            warn!(
                self.ctx.logger(),
                "{:?} is dirty (drawing) - flushing", drawing_region
            );
            self.flush();
        }

        self.drawn_regions.mark(drawing_region);
        self.commands.push(Command::Triangle);
        self.triangles.push(triangle);
    }

    pub fn enqueue_rectangle(&mut self, rectangle: InterfaceRectangle) {
        let rectangle = data::Rectangle::new(rectangle);
        if let Some(sampling_region) = rectangle.texconfig().sampling_region()
            && self.drawn_regions.is_dirty(sampling_region)
        {
            warn!(
                self.ctx.logger(),
                "{:?} is dirty - flushing", sampling_region
            );
            self.flush();

            self.sampled_regions.mark(sampling_region);
        }

        let drawing_region = rectangle.bounding_region();
        if self.sampled_regions.is_dirty(drawing_region) {
            warn!(
                self.ctx.logger(),
                "{:?} is dirty (drawing) - flushing", drawing_region
            );
            self.flush();
        }

        self.drawn_regions.mark(drawing_region);
        self.commands.push(Command::Rectangle);
        self.rectangles.push(rectangle);
    }

    pub fn flush(&mut self) {
        if self.commands.is_empty() {
            return;
        }

        info!(self.ctx.logger(), "flushing rasterizer");
        assert_eq!(
            self.rectangles.len() + self.triangles.len(),
            self.commands.len()
        );

        // config buffer
        let config_buffer =
            self.ctx
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("rasterizer config"),
                    contents: &to_buffer(&self.config),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });

        let config_bind_group = self
            .ctx
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("rasterizer config"),
                layout: &self.config_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &config_buffer,
                        offset: 0,
                        size: None,
                    }),
                }],
            });

        // commands buffer
        let commands_buffer =
            self.ctx
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("commands"),
                    usage: wgpu::BufferUsages::STORAGE,
                    contents: self.commands.as_bytes(),
                });

        // primitives
        let triangle_data = to_buffer(&ShaderSlice::new(&self.triangles));
        let triangles_buffer =
            self.ctx
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("triangles"),
                    usage: wgpu::BufferUsages::STORAGE,
                    contents: &triangle_data,
                });

        let rectangle_data = to_buffer(&ShaderSlice::new(&self.rectangles));
        let rectangles_buffer =
            self.ctx
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("rectangles"),
                    usage: wgpu::BufferUsages::STORAGE,
                    contents: &rectangle_data,
                });

        // bind group
        let rasterizer_bind_group =
            self.ctx
                .device()
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("rasterizer data"),
                    layout: &self.data_bind_group_layout,
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
        pass.set_bind_group(0, &self.vram_bind_group, &[]);
        pass.set_bind_group(1, &config_bind_group, &[]);
        pass.set_bind_group(2, &rasterizer_bind_group, &[]);
        pass.dispatch_workgroups(1024 / 8, 512 / 8, 1);

        std::mem::drop(pass);
        self.ctx.queue().submit([encoder.finish()]);

        self.commands.clear();
        self.triangles.clear();
        self.rectangles.clear();

        self.drawn_regions.clear();
        self.sampled_regions.clear();
    }
}
