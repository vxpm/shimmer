mod display;
mod texture;
mod triangle;
mod util;
mod vram;

use display::DisplayRenderer;
use shimmer_core::gpu::renderer::Action;
use std::sync::{Arc, OnceLock, mpsc::Receiver};
use tinylog::{Logger, debug};
use triangle::TriangleRenderer;
use vram::Vram;
use wgpu::util::DeviceExt;
use zerocopy::IntoBytes;

#[derive(Debug, Clone)]
pub struct Config {
    pub display_tex_format: wgpu::TextureFormat,
}

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
                    texture::TextureBundleView::bind_group_layout(self.device(), sample_type)
                })
            }
            wgpu::TextureSampleType::Depth => todo!(),
            wgpu::TextureSampleType::Sint => todo!(),
            wgpu::TextureSampleType::Uint => self.uint_texbundle_view_layout.get_or_init(|| {
                texture::TextureBundleView::bind_group_layout(self.device(), sample_type)
            }),
        }
    }

    fn device(&self) -> &wgpu::Device {
        &self.device
    }

    fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
}

pub struct Renderer {
    ctx: Context,
    receiver: Receiver<Action>,
    logger: Logger,

    vram: Vram,
    triangle_renderer: TriangleRenderer,
    display_renderer: DisplayRenderer,
}

impl Renderer {
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        receiver: Receiver<Action>,
        logger: Logger,
        config: Config,
    ) -> Self {
        let ctx = Context::new(device, queue, config);
        let vram = Vram::new(&ctx);
        let triangle_renderer = TriangleRenderer::new(&ctx);
        let display_renderer = DisplayRenderer::new(&ctx, vram.texture_bundle().view().clone());

        Self {
            ctx,
            receiver,
            logger,

            vram,
            triangle_renderer,
            display_renderer,
        }
    }

    pub fn prepare(&mut self, _: &wgpu::Device, _: &wgpu::Queue) -> wgpu::CommandBuffer {
        let device = self.ctx.device();
        let queue = self.ctx.queue();

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
                Action::CopyToVram(copy) => {
                    debug!(
                        self.logger,
                        "copying to vram";
                        coords = (copy.x.value(), copy.y.value()),
                        dimensions = (copy.width.value(), copy.height.value())
                    );

                    queue.write_texture(
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
                    queue.submit([]);
                }
                Action::DrawUntexturedTriangle(triangle) => {
                    debug!(
                        self.logger,
                        "rendering untextured triangle";
                        vertices = triangle.vertices,
                        shading_mode = triangle.shading_mode,
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
