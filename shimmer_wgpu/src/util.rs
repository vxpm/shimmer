use crate::context::Context;
use encase::{ShaderSize, ShaderType};
use std::sync::Arc;

/// Helper type for treating a slice of elements as a shader type.
#[derive(ShaderType)]
pub struct ShaderSlice<'a, T: ShaderType + ShaderSize + 'static> {
    #[size(runtime)]
    pub value: &'a [T],
}

impl<'a, T: ShaderType + ShaderSize> ShaderSlice<'a, T> {
    pub fn new(value: &'a [T]) -> Self {
        Self { value }
    }
}

struct BufferPoolInner {
    buffer: wgpu::Buffer,
    taken: bool,
}

pub struct BufferPool {
    ctx: Arc<Context>,
    usage: wgpu::BufferUsages,
    buffers: Vec<BufferPoolInner>,
}

impl BufferPool {
    pub fn new(ctx: Arc<Context>, usage: wgpu::BufferUsages) -> Self {
        Self {
            ctx,
            usage,
            buffers: Vec::new(),
        }
    }

    pub fn get(&mut self, size: u64) -> wgpu::Buffer {
        let available = self
            .buffers
            .iter_mut()
            .filter(|buf| !buf.taken && buf.buffer.size() >= size)
            .min_by_key(|buf| buf.buffer.size());

        match available {
            Some(available) => {
                available.taken = true;
                available.buffer.clone()
            }
            None => {
                let buffer = self.ctx.device().create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size,
                    usage: self.usage,
                    mapped_at_creation: false,
                });

                self.buffers.push(BufferPoolInner {
                    buffer: buffer.clone(),
                    taken: true,
                });

                buffer
            }
        }
    }

    pub fn reclaim(&mut self) {
        self.buffers.iter_mut().for_each(|buf| buf.taken = false);
    }
}
