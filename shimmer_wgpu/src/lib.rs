mod display;
mod texture;
mod triangle;
mod util;
mod vram;

use display::DisplayRenderer;
use shimmer_core::gpu::renderer::Action;
use std::sync::{Arc, Mutex, OnceLock, mpsc::Receiver};
use tinylog::{Logger, debug};
use triangle::TriangleRenderer;
use vram::Vram;
use wgpu::util::DeviceExt;
use zerocopy::IntoBytes;

/// Configuration for the renderer.
#[derive(Debug, Clone)]
pub struct Config {
    pub display_tex_format: wgpu::TextureFormat,
}

/// A context for the renderer: WGPU utilities and it's config.
struct Context {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,

    config: Config,
    float_texbundle_view_layout: OnceLock<wgpu::BindGroupLayout>,
    uint_texbundle_view_layout: OnceLock<wgpu::BindGroupLayout>,
}

impl Context {
    fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>, config: Config) -> Self {
        Self {
            device,
            queue,

            config,
            float_texbundle_view_layout: Default::default(),
            uint_texbundle_view_layout: Default::default(),
        }
    }

    fn texbundle_view_layout(
        &self,
        sample_type: wgpu::TextureSampleType,
    ) -> &wgpu::BindGroupLayout {
        match sample_type {
            wgpu::TextureSampleType::Float { filterable: _ } => {
                self.float_texbundle_view_layout.get_or_init(|| {
                    texture::TextureBundleView::bind_group_layout(&self.device, sample_type)
                })
            }
            wgpu::TextureSampleType::Depth => todo!(),
            wgpu::TextureSampleType::Sint => todo!(),
            wgpu::TextureSampleType::Uint => self.uint_texbundle_view_layout.get_or_init(|| {
                texture::TextureBundleView::bind_group_layout(&self.device, sample_type)
            }),
        }
    }
}

struct Inner {
    ctx: Context,
    logger: Logger,

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
        let ctx = Context::new(device, queue, config);
        let vram = Vram::new(&ctx);
        let triangle_renderer = TriangleRenderer::new(&ctx);
        let display_renderer = DisplayRenderer::new(&ctx, vram.texture_bundle().view().clone());

        let mut encoder = ctx.device.create_command_encoder(&Default::default());
        let pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shimmer_wgpu render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &vram.texture_bundle().view().view(),
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
            logger,

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
        self.ctx.queue.submit([current_encoder.finish()]);

        // create new encoder & pass
        let mut encoder = self.ctx.device.create_command_encoder(&Default::default());
        let pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
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
            })
            .forget_lifetime();

        self.current_encoder = Some(encoder);
        self.current_pass = Some(pass);
    }

    fn exec(&mut self, action: Action) {
        match action {
            Action::SetDisplayResolution(resolution) => {
                debug!(
                    self.logger,
                    "display resolution";
                    horizontal = resolution.horizontal,
                    vertical = resolution.vertical,
                );
                self.display_renderer.set_display_resolution(
                    &self.ctx,
                    resolution.horizontal,
                    resolution.vertical,
                );
            }
            Action::SetDisplayTopLeft(top_left) => {
                debug!(
                    self.logger,
                    "display top left";
                    x = top_left.x,
                    y = top_left.y,
                );
                self.display_renderer
                    .set_display_top_left(&self.ctx, top_left.x, top_left.y);
            }
            Action::CopyToVram(copy) => {
                debug!(
                    self.logger,
                    "copying to vram";
                    coords = (copy.x.value(), copy.y.value()),
                    dimensions = (copy.width.value(), copy.height.value())
                );

                self.flush();
                self.ctx.queue.write_texture(
                    wgpu::ImageCopyTexture {
                        texture: self.vram.texture_bundle().texture(),
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
                    self.logger,
                    "rendering textured triangle";
                    vertices = triangle.vertices,
                    clut = triangle.clut,
                    texpage = triangle.texpage,
                );

                // copy vertices into a buffer
                let buffer =
                    self.ctx
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("triangle"),
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                            contents: triangle.vertices.as_bytes(),
                        });

                let pass = self.current_pass.as_mut().unwrap();
                self.triangle_renderer.render(pass, buffer.slice(..));
            }
            Action::DrawUntexturedTriangle(triangle) => {
                debug!(
                    self.logger,
                    "rendering untextured triangle";
                    vertices = triangle.vertices,
                );

                // copy vertices into a buffer
                let buffer =
                    self.ctx
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("triangle"),
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                            contents: triangle.vertices.as_bytes(),
                        });

                let pass = self.current_pass.as_mut().unwrap();
                self.triangle_renderer.render(pass, buffer.slice(..));
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
