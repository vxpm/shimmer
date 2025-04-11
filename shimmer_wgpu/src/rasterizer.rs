mod data;
mod dirty;

use crate::{
    context::Context,
    util::{BufferPool, ShaderSlice},
    vram::Vram,
};
use data::{Config, to_buffer};
use dirty::DirtyRegions;
use glam::UVec2;
use shimmer::{
    core::gpu::texture::TexWindow,
    gpu::interface::{
        DrawingArea, DrawingSettings, Rectangle as InterfaceRectangle,
        Triangle as InterfaceTriangle,
    },
};
use std::sync::Arc;
use tinylog::{debug, info, trace, warn};
use zerocopy::{Immutable, IntoBytes};

const MAX_SYNCS_PER_VBLANK: u32 = 128;

#[derive(Clone, Copy, PartialEq, Eq, Immutable, IntoBytes)]
#[repr(u32)]
pub enum Command {
    Finish,
    Config,
    Triangle,
    Rectangle,
}

pub struct Rasterizer {
    ctx: Arc<Context>,

    vram_bind_group: wgpu::BindGroup,

    data_bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,

    buffer_pool: BufferPool,
    command_buffers: Vec<wgpu::CommandBuffer>,

    config: Config,

    configs: Vec<Config>,
    commands: Vec<Command>,
    triangles: Vec<data::Triangle>,
    rectangles: Vec<data::Rectangle>,

    syncs: u32,
    drawn_regions: DirtyRegions,
    sampled_regions: DirtyRegions,
}

impl Rasterizer {
    pub fn new(ctx: Arc<Context>, vram: &Vram) -> Self {
        let shader = ctx
            .device()
            .create_shader_module(wgpu::include_wgsl!("../shaders/built/rasterizer.wgsl"));

        let config = Config {
            drawing_area_coords: UVec2::ZERO,
            drawing_area_dimensions: UVec2::new(1024, 512),

            write_to_mask: false as u32,
            check_mask: false as u32,

            texwindow_mask: UVec2::ZERO,
            texwindow_offset: UVec2::ZERO,

            blending_mode: 0,
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
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
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
                    bind_group_layouts: &[vram.bind_group_layout(), &data_bind_group_layout],
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
            vram_bind_group: vram.bind_group().clone(),

            data_bind_group_layout,
            pipeline,

            buffer_pool: BufferPool::new(
                ctx.clone(),
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            ),

            config: config.clone(),
            command_buffers: Vec::new(),

            configs: vec![config],
            commands: Vec::with_capacity(64),
            triangles: Vec::with_capacity(64),
            rectangles: Vec::with_capacity(64),

            syncs: 0,
            drawn_regions: DirtyRegions::default(),
            sampled_regions: DirtyRegions::default(),

            ctx,
        }
    }

    pub fn set_drawing_settings(&mut self, settings: DrawingSettings) {
        trace!(
            self.ctx.logger(),
            "changed drawing settings"; settings = settings
        );

        self.config.blending_mode = settings.blending_mode as u32;
        self.config.write_to_mask = settings.write_to_mask as u32;
        self.config.check_mask = settings.check_mask as u32;

        self.commands.push(Command::Config);
        self.configs.push(self.config.clone());
    }

    pub fn set_drawing_area(&mut self, area: DrawingArea) {
        trace!(
            self.ctx.logger(),
            "changed drawing area"; area = area
        );

        self.config.drawing_area_coords = UVec2::new(
            u32::from(area.coords.x.value()),
            u32::from(area.coords.y.value()),
        );
        self.config.drawing_area_dimensions = UVec2::new(
            u32::from(area.dimensions.width.value()),
            u32::from(area.dimensions.height.value()),
        );

        self.commands.push(Command::Config);
        self.configs.push(self.config.clone());
    }

    pub fn set_texwindow(&mut self, window: TexWindow) {
        trace!(
            self.ctx.logger(),
            "changed texture window"; window = window
        );

        self.config.texwindow_mask = UVec2::new(
            u32::from(window.mask_x().value()),
            u32::from(window.mask_y().value()),
        );
        self.config.texwindow_offset = UVec2::new(
            u32::from(window.offset_x().value()),
            u32::from(window.offset_y().value()),
        );
        self.commands.push(Command::Config);
        self.configs.push(self.config.clone());
    }

    pub fn enqueue_triangle(&mut self, triangle: InterfaceTriangle) {
        debug!(
            self.ctx.logger(),
            "enqueued triangle"; tri = triangle
        );

        let triangle = data::Triangle::new(triangle);
        if let Some(sampling_region) = triangle.texconfig().sampling_region()
            && self.drawn_regions.is_dirty(sampling_region)
        {
            warn!(
                self.ctx.logger(),
                "{:?} is dirty (on triangle sampling) - syncing", sampling_region
            );
            self.sync();

            self.sampled_regions.mark(sampling_region);
        }

        let drawing_region = triangle.bounding_region();
        if self.sampled_regions.is_dirty(drawing_region) {
            warn!(
                self.ctx.logger(),
                "{:?} is dirty (on triangle drawing) - syncing", drawing_region
            );
            self.sync();
        }

        self.drawn_regions.mark(drawing_region);
        self.commands.push(Command::Triangle);
        self.triangles.push(triangle);
    }

    pub fn enqueue_rectangle(&mut self, rectangle: InterfaceRectangle) {
        debug!(
            self.ctx.logger(),
            "enqueued rectangle"; rect = rectangle
        );

        let rectangle = data::Rectangle::new(rectangle);
        if let Some(sampling_region) = rectangle.texconfig().sampling_region()
            && self.drawn_regions.is_dirty(sampling_region)
        {
            warn!(
                self.ctx.logger(),
                "{:?} is dirty (on rectangle sampling) - syncing", sampling_region
            );
            self.sync();

            self.sampled_regions.mark(sampling_region);
        }

        let drawing_region = rectangle.bounding_region();
        if self.sampled_regions.is_dirty(drawing_region) {
            warn!(
                self.ctx.logger(),
                "{:?} is dirty (on rectangle drawing) - syncing", drawing_region
            );
            self.sync();
        }

        self.drawn_regions.mark(drawing_region);
        self.commands.push(Command::Rectangle);
        self.rectangles.push(rectangle);
    }

    pub fn vblank(&mut self) {
        self.syncs = 0;
        self.sync();
        self.flush();
    }

    pub fn flush(&mut self) {
        info!(self.ctx.logger(), "flushing rasterizer");
        self.ctx.queue().submit(self.command_buffers.drain(..));
        self.buffer_pool.reclaim();
    }

    pub fn sync(&mut self) {
        if self.commands.is_empty() {
            return;
        }

        if self.syncs >= MAX_SYNCS_PER_VBLANK {
            warn!(
                self.ctx.logger(),
                "too many synchronization points - ignoring sync request"
            );
            return;
        }

        self.syncs += 1;
        info!(self.ctx.logger(), "synchronizing rasterizer");
        assert_eq!(
            self.rectangles.len() + self.triangles.len() + self.configs.len() - 1,
            self.commands.len()
        );

        let configs_data = to_buffer(&self.configs);
        let configs_buffer = self.buffer_pool.get(configs_data.len() as u64);
        self.ctx
            .queue()
            .write_buffer(&configs_buffer, 0, &configs_data);

        self.commands.push(Command::Finish);
        let commands_data = self.commands.as_bytes();
        let commands_buffer = self.buffer_pool.get(commands_data.len() as u64);
        self.ctx
            .queue()
            .write_buffer(&commands_buffer, 0, &commands_data);

        let triangles_data = to_buffer(&ShaderSlice::new(&self.triangles));
        let triangles_buffer = self.buffer_pool.get(triangles_data.len() as u64);
        self.ctx
            .queue()
            .write_buffer(&triangles_buffer, 0, &triangles_data);

        let rectangles_data = to_buffer(&ShaderSlice::new(&self.rectangles));
        let rectangles_buffer = self.buffer_pool.get(rectangles_data.len() as u64);
        self.ctx
            .queue()
            .write_buffer(&rectangles_buffer, 0, &rectangles_data);

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
                                buffer: &configs_buffer,
                                offset: 0,
                                size: None,
                            }),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: &triangles_buffer,
                                offset: 0,
                                size: None,
                            }),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: &rectangles_buffer,
                                offset: 0,
                                size: None,
                            }),
                        },
                    ],
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
        pass.set_bind_group(0, &self.vram_bind_group, &[]);
        pass.set_bind_group(1, &rasterizer_bind_group, &[]);
        pass.dispatch_workgroups(1024 / 8, 512 / 8, 1);

        std::mem::drop(pass);
        self.command_buffers.push(encoder.finish());

        if self.command_buffers.len() >= 8 {
            self.flush();
        }

        self.commands.clear();
        self.configs.clear();
        self.triangles.clear();
        self.rectangles.clear();

        self.drawn_regions.clear();
        self.sampled_regions.clear();

        self.configs.push(self.config.clone());
    }
}
