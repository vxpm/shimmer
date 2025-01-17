use crate::{Context, texture::TextureBundle};
use zerocopy::IntoBytes;

pub const VRAM_WIDTH: usize = 1024;
pub const VRAM_HEIGHT: usize = 512;

pub struct Vram {
    back: TextureBundle,
    front: TextureBundle,
}

impl Vram {
    pub fn new(ctx: &Context) -> Self {
        let data = vec![0u16; VRAM_WIDTH * VRAM_HEIGHT];
        let back = TextureBundle::new(
            &ctx.device,
            &ctx.queue,
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

        let front = TextureBundle::new(
            &ctx.device,
            &ctx.queue,
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

        Self { back, front }
    }

    pub fn back_texture_bundle(&self) -> &TextureBundle {
        &self.back
    }

    pub fn front_texture_bundle(&self) -> &TextureBundle {
        &self.front
    }

    pub fn sync(&self, ctx: &Context) {
        let mut encoder = ctx.device.create_command_encoder(&Default::default());
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

        ctx.queue.submit([encoder.finish()]);
    }
}
