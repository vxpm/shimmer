mod util;

use shimmer_core::gpu::{
    self,
    cmd::display::DisplayAreaCmd,
    renderer::{self, Action, Rgba},
};
use std::sync::mpsc::Receiver;
use zerocopy::IntoBytes;

fn triangle_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    const ATTRS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Uint8x4, // RGBA
        1 => Sint16x2, // XY
        2 => Uint8x2, // UV
    ];

    wgpu::VertexBufferLayout {
        array_stride: size_of::<gpu::renderer::Vertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ATTRS,
    }
}

struct Shaders {
    /// Shader for flat rendering.
    flat: wgpu::ShaderModule,
    /// Shader for rendering a texture.
    render_texture: wgpu::ShaderModule,
}

impl Shaders {
    fn new(device: &wgpu::Device) -> Self {
        let flat = device.create_shader_module(wgpu::include_wgsl!("../shaders/flat.wgsl"));
        let render_texture =
            device.create_shader_module(wgpu::include_wgsl!("../shaders/render_texture.wgsl"));

        Self {
            flat,
            render_texture,
        }
    }
}

struct BindGroupLayouts {
    render_display: wgpu::BindGroupLayout,
}

impl BindGroupLayouts {
    pub fn new(device: &wgpu::Device) -> Self {
        let render_display = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("render display"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        Self { render_display }
    }
}

struct Pipelines {
    render_display: wgpu::RenderPipeline,
    flat_untextured_triangle: wgpu::RenderPipeline,
}

impl Pipelines {
    fn new(
        device: &wgpu::Device,
        screen_target_format: wgpu::ColorTargetState,
        shaders: &Shaders,
        bind_group_layouts: &BindGroupLayouts,
    ) -> Self {
        let color_target = wgpu::ColorTargetState {
            format: wgpu::TextureFormat::Rgba8Unorm,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        };

        let render_display_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render display layout"),
                bind_group_layouts: &[&bind_group_layouts.render_display],
                push_constant_ranges: &[],
            });

        let render_display = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render display"),
            layout: Some(&render_display_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shaders.render_texture,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shaders.render_texture,
                entry_point: Some("fs_main"),
                targets: &[Some(screen_target_format)],
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

        let triangle_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("triangle"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let flat_untextured_triangle =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("flat untextured triangle"),
                layout: Some(&triangle_layout),
                vertex: wgpu::VertexState {
                    module: &shaders.flat,
                    entry_point: Some("vs_main"),
                    buffers: &[triangle_vertex_layout()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shaders.flat,
                    entry_point: Some("fs_main"),
                    targets: &[Some(color_target)],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
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

        Self {
            flat_untextured_triangle,
            render_display,
        }
    }
}

pub struct Renderer {
    receiver: Receiver<Action>,

    shaders: Shaders,
    pipelines: Pipelines,
    display_area: DisplayAreaCmd,

    // Render display.
    render_display_view: wgpu::TextureView,
    render_display_sampler: wgpu::Sampler,
    render_display_bind_group: wgpu::BindGroup,

    /// VRAM as a 1024x512 RGBA8 texture. This is useful for rendering, but requires intermediate
    /// buffers for bliting.
    vram: wgpu::Texture,
}

impl Renderer {
    pub fn new(
        receiver: Receiver<Action>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen_target_format: wgpu::ColorTargetState,
    ) -> Self {
        let shaders = Shaders::new(device);
        let bind_group_layouts = BindGroupLayouts::new(device);
        let pipelines = Pipelines::new(device, screen_target_format, &shaders, &bind_group_layouts);

        let vram = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("psx vram"),
            size: wgpu::Extent3d {
                width: 1024,
                height: 512,
                depth_or_array_layers: 1,
            },
            usage: wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            view_formats: &[],
        });

        let starting_color = Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        };

        let starting_tex_data = vec![starting_color; 1024 * 512];
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &vram,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: Default::default(),
            },
            starting_tex_data.as_bytes(),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * 1024),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: 1024,
                height: 512,
                depth_or_array_layers: 1,
            },
        );

        let render_display_view = vram.create_view(&wgpu::TextureViewDescriptor::default());
        let render_display_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let render_display_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render display bind group"),
            layout: &bind_group_layouts.render_display,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&render_display_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&render_display_sampler),
                },
            ],
        });

        Self {
            receiver,
            shaders,
            pipelines,
            display_area: DisplayAreaCmd::from_bits(0),

            render_display_view,
            render_display_sampler,
            render_display_bind_group,

            vram,
        }
    }

    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::CommandBuffer {
        let mut encoder = device.create_command_encoder(&Default::default());
        while let Ok(action) = self.receiver.try_recv() {
            match action {
                Action::Reset => (),
                Action::DrawSettings(_) => (),
                Action::DisplayMode(_) => (),
                Action::DisplayArea(_) => (),
                Action::CopyToVram(_) => (),
                Action::DrawUntexturedTriangle(triangle) => {
                    // copy vertices into a buffer
                    let triangle_vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("triangle"),
                        size: 3 * size_of::<renderer::Vertex>() as u64,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });

                    queue.write_buffer(&triangle_vertex_buf, 0, triangle.vertices.as_bytes());
                    queue.submit([]);

                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("untextured triangle rendering"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &self.render_display_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                    render_pass.set_pipeline(&self.pipelines.flat_untextured_triangle);
                    render_pass.set_vertex_buffer(0, triangle_vertex_buf.slice(..));
                    render_pass.draw(0..3, 0..1);
                }
            }
        }

        encoder.finish()
    }

    pub fn render(&mut self, render_pass: &mut wgpu::RenderPass<'_>) {
        render_pass.set_pipeline(&self.pipelines.render_display);
        render_pass.set_bind_group(0, &self.render_display_bind_group, &[]);
        render_pass.draw(0..4, 0..1);
    }
}
