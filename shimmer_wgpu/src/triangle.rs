use shimmer_core::gpu::{
    self,
    cmd::{
        environment::{TexPage, TexPageDepth},
        rendering::Clut,
    },
    renderer::Vertex,
};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use zerocopy::{Immutable, IntoBytes};

use crate::context::{
    Context,
    texture::{R16Uint, TextureBundle},
};

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

#[derive(Debug, Clone, IntoBytes, Immutable)]
#[repr(C)]
struct Extra {
    kind: TexturedKind,
    clut_x: u32,
    clut_y: u32,
    texpage_x: u32,
    texpage_y: u32,
}

pub struct TriangleRenderer {
    ctx: Arc<Context>,

    pipeline: wgpu::RenderPipeline,

    back_vram_bg: wgpu::BindGroup,

    extra_bg_layout: wgpu::BindGroupLayout,
    untextured_extra_bg: wgpu::BindGroup,
}

impl TriangleRenderer {
    pub fn new(ctx: Arc<Context>, back_vram: TextureBundle<R16Uint>) -> Self {
        let shader = ctx
            .device()
            .create_shader_module(wgpu::include_wgsl!("../shaders/built/triangle.wgsl"));

        let back_vram_bg_layout = ctx.texbundle_bind_group_layout::<R16Uint>();
        let extra_bg_layout =
            ctx.device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("textured triangle extra"),
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

        let back_vram_bg = ctx.texbundle_bind_group(&back_vram);
        let untextured_extra = ctx
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("triangle info"),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                contents: Extra {
                    kind: TexturedKind::Untextured,
                    clut_x: 0,
                    clut_y: 0,
                    texpage_x: 0,
                    texpage_y: 0,
                }
                .as_bytes(),
            });

        let untextured_extra_bg = ctx.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("textured triangle info"),
            layout: &extra_bg_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &untextured_extra,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        let pipeline_layout =
            ctx.device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("triangle"),
                    bind_group_layouts: &[back_vram_bg_layout, &extra_bg_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = ctx
            .device()
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
            ctx,

            pipeline,

            back_vram_bg,

            extra_bg_layout,
            untextured_extra_bg,
        }
    }

    pub fn render(&self, ctx: &Context, pass: &mut wgpu::RenderPass, triangle: [Vertex; 3]) {
        // copy vertices into a buffer
        let vertices = ctx
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("triangle vertices"),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                contents: triangle.as_bytes(),
            });

        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.set_bind_group(0, &self.back_vram_bg, &[]);
        pass.set_bind_group(1, &self.untextured_extra_bg, &[]);
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
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("textured triangle vertices"),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                contents: triangle.as_bytes(),
            });

        // copy extra into a buffer
        let extra = Extra {
            kind: match texpage.depth() {
                TexPageDepth::Nibble => TexturedKind::Nibble,
                TexPageDepth::Byte => TexturedKind::Byte,
                TexPageDepth::Full => TexturedKind::Full,
                TexPageDepth::Reserved => TexturedKind::Untextured,
            },
            clut_x: clut.x_by_16().value() as u32 * 16,
            clut_y: clut.y().value() as u32,
            texpage_x: texpage.x_base().value() as u32 * 64,
            texpage_y: texpage.y_base().value() as u32 * 256,
        };

        let extra = ctx
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("textured triangle extra"),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                contents: extra.as_bytes(),
            });

        let extra_bg = ctx.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("textured triangle extra"),
            layout: &self.extra_bg_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &extra,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        // let extra = Box::leak(Box::new(extra));
        // let extra_bg = Box::leak(Box::new(extra_bg));

        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.set_bind_group(0, &self.back_vram_bg, &[]);
        pass.set_bind_group(1, &extra_bg, &[]);
        pass.draw(0..3, 0..1);
    }
}
