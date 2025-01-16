use crate::{Context, texture::TextureBundle};
use zerocopy::IntoBytes;

pub const VRAM_WIDTH: usize = 1024;
pub const VRAM_HEIGHT: usize = 512;

pub struct Vram {
    texture: TextureBundle,
}

impl Vram {
    pub fn new(ctx: &Context) -> Self {
        let data = vec![0u16; VRAM_WIDTH * VRAM_HEIGHT];
        let texture = TextureBundle::new(
            ctx.device(),
            ctx.queue(),
            &wgpu::TextureDescriptor {
                label: Some("psx vram"),
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

        Self { texture }
    }

    pub fn texture_bundle(&self) -> &TextureBundle {
        &self.texture
    }
}
