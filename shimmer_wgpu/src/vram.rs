use crate::texture::TextureBundle;
use shimmer_core::gpu::renderer::Rgba;
use zerocopy::IntoBytes;

pub struct Vram {
    texture: TextureBundle,
}

impl Vram {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let data = vec![Rgba::new(0, 0, 0); 1024 * 512];
        let texture = TextureBundle::new(
            device,
            queue,
            &wgpu::TextureDescriptor {
                label: Some("psx vram"),
                size: wgpu::Extent3d {
                    width: 1024,
                    height: 512,
                    depth_or_array_layers: 1,
                },
                usage: wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
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
