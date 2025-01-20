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

fn vertex_buf_layout() -> wgpu::VertexBufferLayout<'static> {
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

#[derive(Debug, Clone, Copy, IntoBytes, Immutable, Default)]
#[repr(u32)]
enum TexturedKind {
    Nibble,
    Byte,
    Full,
    #[default]
    Untextured,
}

#[derive(Debug, Clone, IntoBytes, Immutable, Default)]
#[repr(C)]
struct TriangleConfig {
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
    config_bg_layout: wgpu::BindGroupLayout,

    vertices: Vec<Vertex>,
    configs: Vec<TriangleConfig>,
}

impl TriangleRenderer {
    pub fn new(ctx: Arc<Context>, back_vram: &TextureBundle<R16Uint>) -> Self {
        let shader = ctx
            .device()
            .create_shader_module(wgpu::include_wgsl!("../shaders/built/triangle.wgsl"));

        let back_vram_bg_layout = ctx.texbundle_bind_group_layout::<R16Uint>();
        let back_vram_bg = ctx.texbundle_bind_group(back_vram);

        let config_bg_layout =
            ctx.device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("textured triangle config"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
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
                    label: Some("triangle"),
                    bind_group_layouts: &[back_vram_bg_layout, &config_bg_layout],
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
                    buffers: &[vertex_buf_layout()],
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
            config_bg_layout,

            vertices: Vec::with_capacity(384),
            configs: Vec::with_capacity(128),
        }
    }

    pub fn push(&mut self, vertices: [Vertex; 3]) {
        self.vertices.extend(vertices);
        self.configs.push(TriangleConfig::default());
    }

    pub fn push_textured(&mut self, vertices: [Vertex; 3], clut: Clut, texpage: TexPage) {
        let config = TriangleConfig {
            kind: match texpage.depth() {
                TexPageDepth::Nibble => TexturedKind::Nibble,
                TexPageDepth::Byte => TexturedKind::Byte,
                TexPageDepth::Full => TexturedKind::Full,
                TexPageDepth::Reserved => TexturedKind::Untextured,
            },
            clut_x: u32::from(clut.x_by_16().value()) * 16,
            clut_y: u32::from(clut.y().value()),
            texpage_x: u32::from(texpage.x_base().value()) * 64,
            texpage_y: u32::from(texpage.y_base().value()) * 256,
        };

        self.vertices.extend(vertices);
        self.configs.push(config);
    }

    pub fn draw(&mut self, pass: &mut wgpu::RenderPass) {
        if self.vertices.is_empty() {
            return;
        }

        let vertex_buf = self
            .ctx
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("triangle vertices"),
                contents: self.vertices.as_bytes(),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let config_buf = self
            .ctx
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("triangle configs"),
                contents: self.configs.as_bytes(),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let config_bg = self
            .ctx
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("triangle configs"),
                layout: &self.config_bg_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &config_buf,
                        offset: 0,
                        size: None,
                    }),
                }],
            });

        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, vertex_buf.slice(..));
        pass.set_bind_group(0, &self.back_vram_bg, &[]);
        pass.set_bind_group(1, &config_bg, &[]);
        pass.draw(0..self.vertices.len() as u32, 0..1);

        self.vertices.clear();
        self.configs.clear();
    }
}
