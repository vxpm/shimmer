use crate::{Context, texture::TextureBundleView};
use bitos::integer::{u9, u10};
use shimmer_core::gpu::{HorizontalResolution, VerticalResolution};
use wgpu::util::DeviceExt;
use zerocopy::IntoBytes;

pub struct DisplayRenderer {
    pipeline: wgpu::RenderPipeline,

    _texbundle_view: TextureBundleView,
    texbundle_view_bg: wgpu::BindGroup,

    top_left: [u16; 2],
    dimensions: [u16; 2],

    display_area: wgpu::Buffer,
    display_area_bg: wgpu::BindGroup,
}

impl DisplayRenderer {
    pub fn new(ctx: &Context, texbundle_view: TextureBundleView) -> Self {
        let shader = ctx
            .device
            .create_shader_module(wgpu::include_wgsl!("../shaders/display.wgsl"));

        let texbundle_view_bg_layout = ctx.texbundle_view_layout(texbundle_view.sample_type());
        let coordinates_bg_layout =
            ctx.device
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

        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("display"),
                bind_group_layouts: &[
                    ctx.texbundle_view_layout(texbundle_view.sample_type()),
                    &coordinates_bg_layout,
                ],
                push_constant_ranges: &[],
            });

        let pipeline = ctx
            .device
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
                        format: ctx.config.display_tex_format,
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
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("display coordinates"),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                contents: &[0u32, 0u32, 0u32, 0u32].as_bytes(),
            });

        let texbundle_view_bg = texbundle_view.bind_group(&ctx.device, texbundle_view_bg_layout);
        let display_area_bg = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
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

        Self {
            pipeline,

            _texbundle_view: texbundle_view,
            texbundle_view_bg,

            top_left: [0; 2],
            dimensions: [0; 2],

            display_area,
            display_area_bg,
        }
    }

    pub fn set_display_top_left(&mut self, ctx: &Context, x: u10, y: u9) {
        self.top_left = [x.value(), y.value()];
        ctx.queue
            .write_buffer(&self.display_area, 0, &self.top_left.as_bytes());
    }

    pub fn set_display_dimensions(
        &mut self,
        ctx: &Context,
        horizontal: HorizontalResolution,
        vertical: VerticalResolution,
    ) {
        self.dimensions = [horizontal.value(), vertical.value()];
        ctx.queue
            .write_buffer(&self.display_area, 4, &self.dimensions.as_bytes());
    }

    pub fn render(&self, pass: &mut wgpu::RenderPass) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.texbundle_view_bg, &[]);
        pass.set_bind_group(1, &self.display_area_bg, &[]);
        pass.draw(0..4, 0..1);
    }
}
