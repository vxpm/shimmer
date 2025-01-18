mod context;
mod display;
mod texture;
mod triangle;
mod util;
mod vram;

pub use context::Config;
use context::Context;
use display::DisplayRenderer;
use shimmer_core::gpu::renderer::Action;
use std::sync::{Arc, Mutex, mpsc::Receiver};
use tinylog::{Logger, debug};
use triangle::TriangleRenderer;
use vram::Vram;

struct Inner {
    ctx: Arc<Context>,

    vram: Vram,
    triangle_renderer: TriangleRenderer,
    display_renderer: DisplayRenderer,

    current_encoder: Option<wgpu::CommandEncoder>,
    current_pass: Option<wgpu::RenderPass<'static>>,
}

impl Inner {
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        logger: Logger,
        config: Config,
    ) -> Self {
        let ctx = Arc::new(Context::new(device, queue, config, logger));

        let vram = Vram::new(ctx.clone());
        let triangle_renderer =
            TriangleRenderer::new(ctx.clone(), vram.back_texture_bundle().clone());
        let display_renderer =
            DisplayRenderer::new(ctx.clone(), vram.front_texture_bundle().clone());

        let mut encoder = ctx.device().create_command_encoder(&Default::default());
        let pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shimmer_wgpu render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &vram.front_texture_bundle().view(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            })
            .forget_lifetime();

        Self {
            ctx,

            vram,
            triangle_renderer,
            display_renderer,

            current_encoder: Some(encoder),
            current_pass: Some(pass),
        }
    }

    fn flush(&mut self) {
        // finish pass
        self.current_pass.take().unwrap();

        // finish encoder & submit
        let current_encoder = self.current_encoder.take().unwrap();
        self.ctx.queue().submit([current_encoder.finish()]);

        // create new encoder & pass
        let mut encoder = self
            .ctx
            .device()
            .create_command_encoder(&Default::default());
        let pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shimmer_wgpu render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.vram.front_texture_bundle().view(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            })
            .forget_lifetime();

        self.current_encoder = Some(encoder);
        self.current_pass = Some(pass);
    }

    fn exec(&mut self, action: Action) {
        match action {
            Action::SetDisplayResolution(resolution) => {
                debug!(
                    self.ctx.logger(),
                    "display resolution";
                    horizontal = resolution.horizontal,
                    vertical = resolution.vertical,
                );
                self.display_renderer
                    .set_display_resolution(resolution.horizontal, resolution.vertical);
            }
            Action::SetDisplayTopLeft(top_left) => {
                debug!(
                    self.ctx.logger(),
                    "display top left";
                    x = top_left.x,
                    y = top_left.y,
                );
                self.display_renderer
                    .set_display_top_left(top_left.x, top_left.y);
            }
            Action::CopyToVram(copy) => {
                debug!(
                    self.ctx.logger(),
                    "copying to vram";
                    coords = (copy.x.value(), copy.y.value()),
                    dimensions = (copy.width.value(), copy.height.value())
                );

                self.flush();
                self.ctx.queue().write_texture(
                    wgpu::ImageCopyTexture {
                        texture: self.vram.front_texture_bundle().texture(),
                        mip_level: 0,
                        origin: wgpu::Origin3d {
                            x: copy.x.value() as u32,
                            y: copy.y.value() as u32,
                            z: 0,
                        },
                        aspect: Default::default(),
                    },
                    &copy.data,
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(copy.width.value() as u32 * 2),
                        rows_per_image: Some(copy.height.value() as u32),
                    },
                    wgpu::Extent3d {
                        width: copy.width.value() as u32,
                        height: copy.height.value() as u32,
                        depth_or_array_layers: 1,
                    },
                );
            }
            Action::DrawTexturedTriangle(triangle) => {
                debug!(
                    self.ctx.logger(),
                    "rendering textured triangle";
                    vertices = triangle.vertices,
                    clut = triangle.clut,
                    texpage = triangle.texpage,
                );

                self.flush();
                self.vram.sync(&self.ctx);

                let pass = self.current_pass.as_mut().unwrap();
                self.triangle_renderer.render_textured(
                    &self.ctx,
                    pass,
                    triangle.vertices,
                    triangle.clut,
                    triangle.texpage,
                );
            }
            Action::DrawUntexturedTriangle(triangle) => {
                debug!(
                    self.ctx.logger(),
                    "rendering untextured triangle";
                    vertices = triangle.vertices,
                );

                let pass = self.current_pass.as_mut().unwrap();
                self.triangle_renderer
                    .render(&self.ctx, pass, triangle.vertices);
            }
        }
    }
}

fn render_thread(inner: Arc<Mutex<Inner>>, receiver: Receiver<Action>) {
    loop {
        let Ok(action) = receiver.recv() else {
            // sender has been dropped
            return;
        };

        {
            let mut renderer = inner.lock().unwrap();
            renderer.exec(action);

            while let Ok(action) = receiver.try_recv() {
                renderer.exec(action);
            }
        }
    }
}

pub struct Renderer {
    inner: Arc<Mutex<Inner>>,
    _thread_handle: std::thread::JoinHandle<()>,
}

impl Renderer {
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        receiver: Receiver<Action>,
        logger: Logger,
        config: Config,
    ) -> Self {
        let inner = Arc::new(Mutex::new(Inner::new(device, queue, logger, config)));
        let _thread_handle = std::thread::spawn({
            let inner = inner.clone();
            move || render_thread(inner, receiver)
        });

        Self {
            inner,
            _thread_handle,
        }
    }

    pub fn render(&mut self, pass: &mut wgpu::RenderPass<'_>) {
        let mut inner = self.inner.lock().unwrap();
        inner.flush();
        inner.display_renderer.render(pass);
    }
}
