mod display;
mod texture;
mod triangle;
mod util;
mod vram;

use display::DisplayRenderer;
use shimmer_core::gpu::renderer::Action;
use std::sync::{OnceLock, mpsc::Receiver};
use tinylog::{Logger, debug};
use triangle::TriangleRenderer;
use vram::Vram;
use wgpu::util::DeviceExt;
use zerocopy::IntoBytes;

struct Context {
    display_tex_format: wgpu::TextureFormat,
    texbundle_view_layout: OnceLock<wgpu::BindGroupLayout>,
}

impl Context {
    pub fn new(display_tex_format: wgpu::TextureFormat) -> Self {
        Self {
            display_tex_format,
            texbundle_view_layout: Default::default(),
        }
    }

    pub fn texbundle_view_layout(&self, device: &wgpu::Device) -> &wgpu::BindGroupLayout {
        self.texbundle_view_layout
            .get_or_init(|| texture::TextureBundleView::bind_group_layout(device))
    }
}

pub struct Renderer {
    context: Context,
    receiver: Receiver<Action>,
    logger: Logger,

    vram: Vram,
    triangle_renderer: TriangleRenderer,
    display_renderer: DisplayRenderer,
}

impl Renderer {
    pub fn new(
        receiver: Receiver<Action>,
        logger: Logger,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        display_target_format: wgpu::TextureFormat,
    ) -> Self {
        let context = Context::new(display_target_format);
        let vram = Vram::new(device, queue);
        let triangle_renderer = TriangleRenderer::new(device);
        let display_renderer =
            DisplayRenderer::new(device, &context, vram.texture_bundle().view().clone());

        Self {
            context,
            receiver,
            logger,

            vram,
            triangle_renderer,
            display_renderer,
        }
    }

    pub fn prepare(&mut self, device: &wgpu::Device, _queue: &wgpu::Queue) -> wgpu::CommandBuffer {
        let mut encoder = device.create_command_encoder(&Default::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shimmer_wgpu render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.vram.texture_bundle().view().view(),
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

        while let Ok(action) = self.receiver.try_recv() {
            match action {
                Action::Reset => (),
                Action::DrawSettings(_) => (),
                Action::DisplayMode(_) => (),
                Action::DisplayArea(_) => (),
                Action::CopyToVram(_) => (),
                Action::DrawUntexturedTriangle(triangle) => {
                    debug!(
                        self.logger,
                        "rendering untextured triangle: {:#?}",
                        triangle.clone()
                    );

                    // copy vertices into a buffer
                    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("triangle"),
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        contents: triangle.vertices.as_bytes(),
                    });

                    self.triangle_renderer.render(&mut pass, buffer.slice(..));
                }
            }
        }

        std::mem::drop(pass);
        encoder.finish()
    }

    pub fn render(&mut self, pass: &mut wgpu::RenderPass<'_>) {
        self.display_renderer.render(pass);
    }
}
