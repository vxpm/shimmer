use std::sync::Arc;

use crate::{
    Context,
    context::texture::{R16Uint, TextureBundle},
    util::Rect,
};
use zerocopy::IntoBytes;

pub const VRAM_WIDTH: usize = 1024;
pub const VRAM_HEIGHT: usize = 512;

pub struct Vram {
    ctx: Arc<Context>,

    back: TextureBundle<R16Uint>,
    front: TextureBundle<R16Uint>,
}

impl Vram {
    pub fn new(ctx: Arc<Context>) -> Self {
        let data = vec![0u16; VRAM_WIDTH * VRAM_HEIGHT];
        let back = ctx.create_texbundle(
            &wgpu::TextureDescriptor {
                label: Some("psx vram back buffer"),
                size: wgpu::Extent3d {
                    width: VRAM_WIDTH as u32,
                    height: VRAM_HEIGHT as u32,
                    depth_or_array_layers: 1,
                },
                usage: wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R16Uint,
                view_formats: &[],
            },
            data.as_bytes(),
        );

        let front = ctx.create_texbundle(
            &wgpu::TextureDescriptor {
                label: Some("psx vram front buffer"),
                size: wgpu::Extent3d {
                    width: VRAM_WIDTH as u32,
                    height: VRAM_HEIGHT as u32,
                    depth_or_array_layers: 1,
                },
                usage: wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R16Uint,
                view_formats: &[],
            },
            data.as_bytes(),
        );

        Self { ctx, back, front }
    }

    pub fn back_texbundle(&self) -> &TextureBundle<R16Uint> {
        &self.back
    }

    pub fn front_texbundle(&self) -> &TextureBundle<R16Uint> {
        &self.front
    }

    pub fn sync(&self) {
        let mut encoder = self
            .ctx
            .device()
            .create_command_encoder(&Default::default());
        encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: self.front.texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: Default::default(),
            },
            wgpu::ImageCopyTexture {
                texture: self.back.texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: Default::default(),
            },
            wgpu::Extent3d {
                width: 1024,
                height: 512,
                depth_or_array_layers: 1,
            },
        );

        self.ctx.queue().submit([encoder.finish()]);
    }
}

/// Helper struct for keeping track of dirty VRAM regions.
#[derive(Debug, Default)]
pub struct Dirty {
    rects: Vec<Rect>,
}

impl Dirty {
    pub fn mark(&mut self, rect: Rect) {
        if self.rects.iter().any(|r| r.contains_rect(rect)) {
            return;
        }

        self.rects.push(rect);
    }

    pub fn clear(&mut self) {
        self.rects.clear();
    }

    pub fn is_dirty(&mut self, rect: Rect) -> bool {
        self.rects.iter().any(|r| r.overlaps(rect))
    }
}
