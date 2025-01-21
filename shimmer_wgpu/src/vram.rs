use std::sync::Arc;

use crate::{
    Context,
    context::texture::{R16Uint, TextureBundle},
};
use bitvec::BitArr;
use zerocopy::IntoBytes;

pub const VRAM_WIDTH: u16 = 1024;
pub const VRAM_HEIGHT: u16 = 512;

pub struct Vram {
    ctx: Arc<Context>,

    back: TextureBundle<R16Uint>,
    front: TextureBundle<R16Uint>,
}

impl Vram {
    pub fn new(ctx: Arc<Context>) -> Self {
        let data = vec![0u16; VRAM_WIDTH as usize * VRAM_HEIGHT as usize];
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

const DIRTY_REGION_LEN: u16 = 32;
type Regions = BitArr!(for ((1024 / DIRTY_REGION_LEN) * (1024 / DIRTY_REGION_LEN)) as usize);

#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    top_left: (u16, u16),
    dimensions: (u16, u16),
}

impl Rect {
    pub fn new(top_left: (u16, u16), dimensions: (u16, u16)) -> Self {
        Self {
            top_left,
            dimensions,
        }
    }

    pub fn from_extremes(top_left: (u16, u16), bottom_right: (u16, u16)) -> Self {
        Self {
            top_left,
            dimensions: (bottom_right.0 - top_left.0, bottom_right.1 - top_left.1),
        }
    }
}

/// Helper struct for keeping track of dirty VRAM regions.
#[derive(Debug, Default)]
pub struct Dirty {
    regions: Regions,
}

impl Dirty {
    /// Marks a rectangular region in VRAM as dirty.
    pub fn mark(&mut self, rect: Rect) {
        if rect.dimensions.0 == 0 || rect.dimensions.1 == 0 {
            return;
        }

        let start_x = rect.top_left.0 / DIRTY_REGION_LEN;
        let end_x = (rect.top_left.0 + rect.dimensions.0 - 1) / DIRTY_REGION_LEN;
        let start_y = (rect.top_left.1) / DIRTY_REGION_LEN;
        let end_y = (rect.top_left.1 + rect.dimensions.1 - 1) / DIRTY_REGION_LEN;

        self.regions.set(
            (start_y * (VRAM_WIDTH / DIRTY_REGION_LEN) + start_x) as usize,
            true,
        );

        for y in start_y..=end_y {
            for x in start_x..=end_x {
                self.regions
                    .set((y * (VRAM_WIDTH / DIRTY_REGION_LEN) + x) as usize, true);
            }
        }
    }

    /// Unmarks all dirty regions.
    pub fn clear(&mut self) {
        self.regions.fill(false);
    }

    /// Checks whether a given rectangular region in VRAM is dirty.
    pub fn is_dirty(&mut self, rect: Rect) -> bool {
        if rect.dimensions.0 == 0 || rect.dimensions.1 == 0 {
            return false;
        }

        let start_x = rect.top_left.0 / DIRTY_REGION_LEN;
        let end_x = (rect.top_left.0 + rect.dimensions.0 - 1) / DIRTY_REGION_LEN;
        let start_y = (rect.top_left.1) / DIRTY_REGION_LEN;
        let end_y = (rect.top_left.1 + rect.dimensions.1 - 1) / DIRTY_REGION_LEN;

        for y in start_y..=end_y {
            for x in start_x..=end_x {
                if *self
                    .regions
                    .get((y * (VRAM_WIDTH / DIRTY_REGION_LEN) + x) as usize)
                    .unwrap()
                {
                    return true;
                }
            }
        }

        false
    }
}
