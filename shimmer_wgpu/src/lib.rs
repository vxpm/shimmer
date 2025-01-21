mod context;
mod display;
mod rectangle;
mod texture;
mod triangle;
mod vram;

pub use context::Config;
use context::Context;
use display::DisplayRenderer;
use rectangle::RectangleRenderer;
use shimmer_core::gpu::renderer::{Command, Vertex};
use std::sync::{Arc, Mutex, mpsc::Receiver};
use tinylog::{Logger, debug, warn};
use triangle::TriangleRenderer;
use vram::{Rect, Vram};
use zerocopy::{Immutable, IntoBytes};

#[derive(Debug, Clone, Copy, IntoBytes, Immutable, Default)]
#[repr(u32)]
enum TextureKind {
    #[default]
    Untextured,
    Nibble,
    Byte,
    Full,
}

impl From<shimmer_core::gpu::cmd::environment::TexPageDepth> for TextureKind {
    fn from(value: shimmer_core::gpu::cmd::environment::TexPageDepth) -> Self {
        match value {
            shimmer_core::gpu::cmd::environment::TexPageDepth::Nibble => Self::Nibble,
            shimmer_core::gpu::cmd::environment::TexPageDepth::Byte => Self::Byte,
            shimmer_core::gpu::cmd::environment::TexPageDepth::Full => Self::Full,
            shimmer_core::gpu::cmd::environment::TexPageDepth::Reserved => Self::Full,
        }
    }
}

fn triangle_bounding_rect(vertices: &[Vertex; 3]) -> Rect {
    let mut min_x = u16::MAX;
    let mut max_x = u16::MIN;
    let mut min_y = u16::MAX;
    let mut max_y = u16::MIN;

    for vertex in vertices {
        min_x = min_x.min(vertex.x.value().clamp(0, i16::MAX) as u16);
        max_x = max_x.max(vertex.x.value().clamp(0, i16::MAX) as u16);

        min_y = min_y.min(vertex.y.value().clamp(0, i16::MAX) as u16);
        max_y = max_y.max(vertex.y.value().clamp(0, i16::MAX) as u16);
    }

    Rect::from_extremes((min_x, min_y), (max_x, max_y))
}

struct Inner {
    ctx: Arc<Context>,

    vram: Vram,
    vram_dirty: vram::Dirty,

    triangle_renderer: TriangleRenderer,
    rectangle_renderer: RectangleRenderer,
    display_renderer: DisplayRenderer,
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
        let triangle_renderer = TriangleRenderer::new(ctx.clone(), vram.back_texbundle());
        let rectangle_renderer = RectangleRenderer::new(ctx.clone(), vram.back_texbundle());
        let display_renderer = DisplayRenderer::new(ctx.clone(), vram.front_texbundle());

        Self {
            ctx,

            vram,
            vram_dirty: vram::Dirty::default(),

            triangle_renderer,
            rectangle_renderer,
            display_renderer,
        }
    }

    /// Flushes all pending rendering actions.
    fn flush(&mut self) {
        let mut encoder = self
            .ctx
            .device()
            .create_command_encoder(&Default::default());

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shimmer_wgpu render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: self.vram.front_texbundle().view(),
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

        self.triangle_renderer.draw(&mut pass);
        self.rectangle_renderer.draw(&mut pass);

        std::mem::drop(pass);
        self.ctx.queue().submit([encoder.finish()]);
    }

    fn exec(&mut self, command: Command) {
        match command {
            Command::SetDisplayResolution(resolution) => {
                debug!(
                    self.ctx.logger(),
                    "display resolution";
                    horizontal = resolution.horizontal,
                    vertical = resolution.vertical,
                );

                self.display_renderer
                    .set_display_resolution(resolution.horizontal, resolution.vertical);
            }
            Command::SetDisplayTopLeft(top_left) => {
                debug!(
                    self.ctx.logger(),
                    "display top left";
                    x = top_left.x,
                    y = top_left.y,
                );

                self.display_renderer
                    .set_display_top_left(top_left.x, top_left.y);
            }
            Command::Vsync => {
                self.flush();
            }
            Command::CopyToVram(copy) => {
                debug!(
                    self.ctx.logger(),
                    "copying to vram";
                    coords = (copy.x.value(), copy.y.value()),
                    dimensions = (copy.width.value(), copy.height.value())
                );

                let rect = Rect::new(
                    (copy.x.value(), copy.y.value()),
                    (copy.width.value(), copy.height.value()),
                );
                self.vram_dirty.mark(rect);

                self.ctx.queue().write_texture(
                    wgpu::ImageCopyTexture {
                        texture: self.vram.front_texbundle().texture(),
                        mip_level: 0,
                        origin: wgpu::Origin3d {
                            x: u32::from(copy.x.value()),
                            y: u32::from(copy.y.value()),
                            z: 0,
                        },
                        aspect: Default::default(),
                    },
                    &copy.data,
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(u32::from(copy.width.value()) * 2),
                        rows_per_image: Some(u32::from(copy.height.value())),
                    },
                    wgpu::Extent3d {
                        width: u32::from(copy.width.value()),
                        height: u32::from(copy.height.value()),
                        depth_or_array_layers: 1,
                    },
                );
            }
            Command::DrawTriangle(triangle) => {
                match triangle.texture {
                    Some(config) => {
                        debug!(
                            self.ctx.logger(),
                            "rendering textured triangle";
                            vertices = triangle.vertices,
                            clut = config.clut,
                            texpage = config.texpage,
                        );

                        let texpage_rect = Rect::new(
                            (
                                u16::from(config.texpage.x_base().value()) * 64,
                                u16::from(config.texpage.y_base().value()) * 256,
                            ),
                            (64, 256),
                        );

                        if self.vram_dirty.is_dirty(texpage_rect) {
                            warn!(self.ctx.logger(), "DIRTY: {texpage_rect:?}");
                            self.flush();
                            self.vram.sync();
                            self.vram_dirty.clear();
                        }
                    }
                    None => {
                        debug!(
                            self.ctx.logger(),
                            "rendering untextured triangle";
                            vertices = triangle.vertices,
                        );
                    }
                }

                let rect = triangle_bounding_rect(&triangle.vertices);
                self.vram_dirty.mark(rect);
                self.triangle_renderer.enqueue(triangle);
            }
            Command::DrawRectangle(rectangle) => {
                debug!(
                    self.ctx.logger(),
                    "rendering rectangle";
                );

                if let Some(config) = rectangle.texture {
                    let texpage_rect = Rect::new(
                        (
                            u16::from(config.texpage.x_base().value()) * 64,
                            u16::from(config.texpage.y_base().value()) * 256,
                        ),
                        (64, 256),
                    );

                    if self.vram_dirty.is_dirty(texpage_rect) {
                        self.flush();
                        self.vram.sync();
                        self.vram_dirty.clear();
                    }
                }

                let rect = Rect::new(
                    (
                        rectangle.x.value().clamp(0, i16::MAX) as u16,
                        rectangle.y.value().clamp(0, i16::MAX) as u16,
                    ),
                    (rectangle.width.value(), rectangle.height.value()),
                );
                self.vram_dirty.mark(rect);
                self.rectangle_renderer.enqueue(rectangle);
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
        receiver: Receiver<Command>,
        logger: Logger,
        config: Config,
    ) -> Self {
        let inner = Arc::new(Mutex::new(Inner::new(device, queue, logger, config)));
        let _thread_handle = std::thread::Builder::new()
            .name("shimmer_wgpu renderer".to_owned())
            .spawn({
                let inner = inner.clone();
                move || {
                    loop {
                        let Ok(command) = receiver.recv() else {
                            // sender has been dropped
                            return;
                        };

                        {
                            let mut renderer = inner.lock().unwrap();
                            renderer.exec(command);

                            while let Ok(action) = receiver.try_recv() {
                                renderer.exec(action);
                            }
                        }
                    }
                }
            })
            .unwrap();

        Self {
            inner,
            _thread_handle,
        }
    }

    pub fn render(&mut self, pass: &mut wgpu::RenderPass<'_>) {
        let inner = self.inner.lock().unwrap();
        inner.display_renderer.render(pass);
    }
}
