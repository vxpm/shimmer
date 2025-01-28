use std::sync::Arc;
use tinylog::Logger;

/// Configuration for the renderer.
#[derive(Debug, Clone)]
pub struct Config {
    pub display_tex_format: wgpu::TextureFormat,
}

/// A context for the renderer.
pub struct Context {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    config: Config,
    logger: Logger,
}

impl Context {
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        config: Config,
        logger: Logger,
    ) -> Self {
        Self {
            device,
            queue,
            config,
            logger,
        }
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn logger(&self) -> &Logger {
        &self.logger
    }
}
