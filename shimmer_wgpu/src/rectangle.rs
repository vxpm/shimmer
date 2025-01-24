use bitos::integer::{i11, u10};
use shimmer_core::gpu::renderer::{Rectangle, Rgba8};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use zerocopy::{Immutable, IntoBytes};

use crate::{
    TextureKind,
    context::{
        Context,
        texture::{R16Uint, TextureBundle},
    },
};

#[derive(Debug, Clone, IntoBytes, Immutable, Default)]
#[repr(C)]
struct RectangleConfig {
    color: Rgba8,
    x: i11,
    y: i11,
    width: u10,
    height: u10,
    kind: TextureKind,
    clut_x: u10,
    clut_y: u10,
    texpage_x: u10,
    texpage_y: u10,
    u: u8,
    v: u8,
    _padding: u16,
}

impl RectangleConfig {
    fn vertex_buf_layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRS: [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![
            0 => Uint8x4, // RGBA
            1 => Sint16x2, // XY
            2 => Uint16x2, // WIDTH HEIGHT
            3 => Uint32, // TEXTURE KIND
            4 => Uint16x2, // CLUT
            5 => Uint16x2, // TEXPAGE COORDS
            6 => Uint8x2, // UV
        ];

        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &ATTRS,
        }
    }
}

pub struct RectangleRenderer {
    ctx: Arc<Context>,
    pipeline: wgpu::RenderPipeline,
    back_vram_bg: wgpu::BindGroup,
    configs: Vec<RectangleConfig>,
}

impl RectangleRenderer {
    pub fn new(ctx: Arc<Context>, back_vram: &TextureBundle<R16Uint>) -> Self {
        let shader = ctx
            .device()
            .create_shader_module(wgpu::include_wgsl!("../shaders/built/rectangle.wgsl"));

        let back_vram_bg_layout = ctx.texbundle_bind_group_layout::<R16Uint>();
        let back_vram_bg = ctx.texbundle_bind_group(back_vram);

        let pipeline_layout =
            ctx.device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("rectangle"),
                    bind_group_layouts: &[back_vram_bg_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = ctx
            .device()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("rectangle"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[RectangleConfig::vertex_buf_layout()],
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
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
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
            configs: Vec::with_capacity(128),
        }
    }

    /// Enqueues the given rectangle to be drawn.
    pub fn enqueue(&mut self, rect: &Rectangle) {
        let config = match rect.texture {
            Some(config) => RectangleConfig {
                color: rect.color,
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
                kind: config.texpage.depth().into(),
                clut_x: u10::new(u16::from(config.clut.x_by_16().value()) * 16),
                clut_y: u10::new(config.clut.y().value()),
                texpage_x: u10::new(u16::from(config.texpage.x_base().value()) * 64),
                texpage_y: u10::new(u16::from(config.texpage.y_base().value()) * 256),
                u: rect.u,
                v: rect.v,
                _padding: 0,
            },
            None => RectangleConfig {
                color: rect.color,
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
                ..Default::default()
            },
        };

        self.configs.push(config);
    }

    /// Draws the queued rectangles.
    pub fn draw(&mut self, pass: &mut wgpu::RenderPass) {
        if self.configs.is_empty() {
            return;
        }

        let configs_buf = self
            .ctx
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("rectangle configs"),
                contents: self.configs.as_bytes(),
                usage: wgpu::BufferUsages::VERTEX,
            });

        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, configs_buf.slice(..));
        pass.set_bind_group(0, &self.back_vram_bg, &[]);
        pass.draw(0..4, 0..self.configs.len() as u32);

        self.configs.clear();
    }
}
