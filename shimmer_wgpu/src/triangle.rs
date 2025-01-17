use crate::{Context, texture::TextureBundleView};
use shimmer_core::gpu::{
    self,
    cmd::{
        environment::{TexPage, TexPageDepth},
        rendering::Clut,
    },
    renderer::Vertex,
};
use wgpu::util::DeviceExt;
use zerocopy::{Immutable, IntoBytes};

fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
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

#[derive(Debug, Clone, Copy, IntoBytes, Immutable)]
#[repr(u32)]
enum TexturedKind {
    Nibble,
    Byte,
    Full,
    Untextured,
}

#[repr(C, align(8))]
#[derive(Debug, Clone, IntoBytes, Immutable)]
struct TexturedInfo {
    kind: TexturedKind,
    clut_x: u32,
    clut_y: u32,
    texpage_x: u32,
    texpage_y: u32,
    _padding: u32,
}

pub struct TriangleRenderer {
    pipeline: wgpu::RenderPipeline,

    texture_bg: wgpu::BindGroup,

    textured_info_bg_layout: wgpu::BindGroupLayout,
    untextured_info_bg: wgpu::BindGroup,
}

impl TriangleRenderer {
    pub fn new(ctx: &Context, texture: TextureBundleView) -> Self {
        let shader = ctx
            .device
            .create_shader_module(wgpu::include_wgsl!("../shaders/triangle.wgsl"));

        let texture_bg_layout = ctx.texbundle_view_layout(wgpu::TextureSampleType::Uint);
        let textured_info_bg_layout =
            ctx.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("textured triangle info"),
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

        let texture_bg = texture.bind_group(&ctx.device, texture_bg_layout);

        let untextured_info = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("triangle info"),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                contents: TexturedInfo {
                    kind: TexturedKind::Untextured,
                    clut_x: 0,
                    clut_y: 0,
                    texpage_x: 0,
                    texpage_y: 0,
                    _padding: 0,
                }
                .as_bytes(),
            });

        let untextured_info_bg = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("textured triangle info"),
            layout: &textured_info_bg_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &untextured_info,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("triangle"),
                bind_group_layouts: &[
                    ctx.texbundle_view_layout(wgpu::TextureSampleType::Uint),
                    &textured_info_bg_layout,
                ],
                push_constant_ranges: &[],
            });

        let pipeline = ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("triangle"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[vertex_layout()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::R16Uint,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
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
            pipeline,

            texture_bg,

            textured_info_bg_layout,
            untextured_info_bg,
        }
    }

    pub fn render(&self, ctx: &Context, pass: &mut wgpu::RenderPass, triangle: [Vertex; 3]) {
        // copy vertices into a buffer
        let vertices = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("triangle vertices"),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                contents: triangle.as_bytes(),
            });

        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.set_bind_group(0, &self.texture_bg, &[]);
        pass.set_bind_group(1, &self.untextured_info_bg, &[]);
        pass.draw(0..3, 0..1);
    }

    pub fn render_textured(
        &self,
        ctx: &Context,
        pass: &mut wgpu::RenderPass,
        triangle: [Vertex; 3],
        clut: Clut,
        texpage: TexPage,
    ) {
        // copy vertices into a buffer
        let vertices = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("textured triangle vertices"),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                contents: triangle.as_bytes(),
            });

        // copy textured info into a buffer
        let textured_info = TexturedInfo {
            kind: match texpage.depth() {
                TexPageDepth::Nibble => TexturedKind::Nibble,
                TexPageDepth::Byte => TexturedKind::Byte,
                TexPageDepth::Full => TexturedKind::Full,
                TexPageDepth::Reserved => TexturedKind::Untextured,
            },
            clut_x: clut.x_by_16().value() as u32 * 16,
            clut_y: clut.y().value() as u32,
            texpage_x: texpage.x_base().value() as u32,
            texpage_y: texpage.y_base().value() as u32,
            _padding: 0,
        };

        let textured_info = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("textured triangle info"),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                contents: textured_info.as_bytes(),
            });

        let textured_info_bg = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("textured triangle info"),
            layout: &self.textured_info_bg_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &textured_info,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.set_bind_group(0, &self.texture_bg, &[]);
        pass.set_bind_group(1, &textured_info_bg, &[]);
        pass.draw(0..3, 0..1);
    }
}
