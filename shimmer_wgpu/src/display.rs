use std::sync::Arc;

use crate::{Context, vram::Vram};
use bitos::integer::{u9, u10};
use shimmer::core::gpu::{HorizontalResolution, VerticalResolution};
use wgpu::util::DeviceExt;
use zerocopy::IntoBytes;

pub struct DisplayRenderer {
    ctx: Arc<Context>,

    pipeline: wgpu::RenderPipeline,
    vram_bind_group: wgpu::BindGroup,

    top_left: [u16; 2],
    dimensions: [u16; 2],

    display_area: wgpu::Buffer,
    display_area_bg: wgpu::BindGroup,
    all_of_vram_bg: wgpu::BindGroup,
}

impl DisplayRenderer {
    pub fn new(ctx: Arc<Context>, vram: &Vram) -> Self {
        let shader = ctx
            .device()
            .create_shader_module(wgpu::include_wgsl!("../shaders/built/display.wgsl"));

        let coordinates_bg_layout =
            ctx.device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("display"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let pipeline_layout =
            ctx.device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("display"),
                    bind_group_layouts: &[vram.bind_group_layout(), &coordinates_bg_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = ctx
            .device()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("display"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: ctx.config().display_tex_format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            });

        let display_area = ctx
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("display coordinates"),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                contents: [0u32, 0u32].as_bytes(),
            });

        let all_of_vram = ctx
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("display coordinates"),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                contents: [0u32, (512 << 16) | 1024].as_bytes(),
            });

        let display_area_bg = ctx.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("display coordinates"),
            layout: &coordinates_bg_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &display_area,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        let all_of_vram_bg = ctx.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("display coordinates (all of vram)"),
            layout: &coordinates_bg_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &all_of_vram,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        Self {
            ctx,

            pipeline,
            vram_bind_group: vram.bind_group().clone(),

            top_left: [0; 2],
            dimensions: [0; 2],

            display_area,
            display_area_bg,
            all_of_vram_bg,
        }
    }

    pub fn set_display_top_left(&mut self, x: u10, y: u9) {
        self.top_left = [x.value(), y.value()];

        self.ctx
            .queue()
            .write_buffer(&self.display_area, 0, self.top_left.as_bytes());
    }

    pub fn set_display_resolution(
        &mut self,
        horizontal: HorizontalResolution,
        vertical: VerticalResolution,
    ) {
        self.dimensions = [horizontal.value(), vertical.value()];

        self.ctx
            .queue()
            .write_buffer(&self.display_area, 4, self.dimensions.as_bytes());
    }

    pub fn render(&self, pass: &mut wgpu::RenderPass) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.vram_bind_group, &[]);
        pass.set_bind_group(1, &self.display_area_bg, &[]);
        pass.draw(0..4, 0..1);
    }

    pub fn render_all(&self, pass: &mut wgpu::RenderPass) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.vram_bind_group, &[]);
        pass.set_bind_group(1, &self.all_of_vram_bg, &[]);
        pass.draw(0..4, 0..1);
    }
}
