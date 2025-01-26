mod context;
mod display;
mod rasterizer;
mod vram;

use context::Context;
use display::DisplayRenderer;
use rasterizer::Rasterizer;
use shimmer_core::gpu::renderer::{Command, Renderer};
use std::sync::{
    Arc, Mutex,
    mpsc::{Sender, channel},
};
use tinylog::Logger;
use vram::Vram;
use zerocopy::{Immutable, IntoBytes};

pub use context::Config;

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

struct Inner {
    ctx: Arc<Context>,

    vram: Vram,
    rasterizer: Rasterizer,
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
        let rasterizer = Rasterizer::new(ctx.clone(), &vram);
        let display_renderer = DisplayRenderer::new(ctx.clone(), &vram);

        Self {
            ctx,

            vram,
            rasterizer,
            display_renderer,
        }
    }

    fn exec(&mut self, command: Command) {
        match command {
            Command::VBlank => {
                println!("VBlank");
                self.rasterizer.flush();
            }
            Command::DrawTriangle(triangle) => {
                self.rasterizer.enqueue(triangle);
            }
            Command::SetDisplayTopLeft(display_top_left) => {
                self.display_renderer
                    .set_display_top_left(display_top_left.x, display_top_left.y);
            }
            Command::SetDisplayResolution(display_resolution) => {
                self.display_renderer.set_display_resolution(
                    display_resolution.horizontal,
                    display_resolution.vertical,
                );
            }
            Command::CopyFromVram(copy) => {
                copy.response.send(Vec::new()).unwrap();
            }
            _ => (),
        }
    }
}

/// A WGPU based renderer implementation.
///
/// This type is reference counted and therefore cheaply clonable.
#[derive(Clone)]
pub struct WgpuRenderer {
    inner: Arc<Mutex<Inner>>,
    sender: Sender<Command>,
}

impl WgpuRenderer {
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        logger: Logger,
        config: Config,
    ) -> Self {
        let inner = Arc::new(Mutex::new(Inner::new(device, queue, logger, config)));
        let (sender, receiver) = channel();

        std::thread::Builder::new()
            .name("shimmer_wgpu renderer".into())
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

        Self { inner, sender }
    }

    pub fn render(&self, pass: &mut wgpu::RenderPass<'_>) {
        let inner = self.inner.lock().unwrap();
        inner.display_renderer.render(pass);
    }
}

impl Renderer for WgpuRenderer {
    fn exec(&mut self, command: Command) {
        self.sender
            .send(command)
            .expect("rendering thread is alive");
    }
}
