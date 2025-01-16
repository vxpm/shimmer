use crate::{Context, texture::TextureBundleView};

pub struct DisplayRenderer {
    pipeline: wgpu::RenderPipeline,

    texbundle_view: TextureBundleView,
    texbundle_view_bg: wgpu::BindGroup,

    coordinates: wgpu::Buffer,
    coordinates_bg: wgpu::BindGroup,
}

impl DisplayRenderer {
    pub fn new(device: &wgpu::Device, ctx: &Context, texbundle_view: TextureBundleView) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/display.wgsl"));
        let coordinates_bg_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("display"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("display"),
            bind_group_layouts: &[ctx.texbundle_view_layout(device), &coordinates_bg_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render display"),
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
                    format: ctx.display_tex_format,
                    blend: Some(wgpu::BlendState::REPLACE),
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

        let coordinates = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("display coordinates"),
            size: 4,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let texbundle_view_bg =
            texbundle_view.bind_group(device, ctx.texbundle_view_layout(device));

        let coordinates_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("display coordinates"),
            layout: &coordinates_bg_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &coordinates,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        Self {
            pipeline,

            texbundle_view,
            texbundle_view_bg,

            coordinates,
            coordinates_bg,
        }
    }

    pub fn render(&self, pass: &mut wgpu::RenderPass) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.texbundle_view_bg, &[]);
        pass.set_bind_group(1, &self.coordinates_bg, &[]);
        pass.draw(0..4, 0..1);
    }
}
