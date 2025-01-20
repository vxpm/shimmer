pub mod texture;

use std::{marker::PhantomData, sync::Arc};
use texture::{TexBundleBgLayouts, TextureBundle, TextureBundleInner};
use tinylog::Logger;
use wgpu::util::DeviceExt;

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

    texbundle_bg_layouts: TexBundleBgLayouts,
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

            texbundle_bg_layouts: Default::default(),
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

    pub fn create_texbundle<F: texture::TexBundleFormat>(
        &self,
        descriptor: &wgpu::TextureDescriptor,
        data: &[u8],
    ) -> TextureBundle<F> {
        assert_eq!(
            descriptor.format.sample_type(None, None),
            F::FORMAT.sample_type(None, None)
        );

        let texture =
            self.device
                .create_texture_with_data(&self.queue, descriptor, Default::default(), data);
        let view = texture.create_view(&Default::default());
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: descriptor.label,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        TextureBundle(Arc::new(TextureBundleInner {
            texture,
            view,
            sampler,
            _phantom: PhantomData,
        }))
    }

    pub fn texbundle_bind_group_layout<F: texture::TexBundleFormat>(
        &self,
    ) -> &wgpu::BindGroupLayout {
        match F::FORMAT {
            wgpu::TextureFormat::R16Uint => self.texbundle_bg_layouts.r16uint.get_or_init(|| {
                self.device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: Some("texture bundle view"),
                        entries: &[
                            wgpu::BindGroupLayoutEntry {
                                binding: 0,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    multisampled: false,
                                    view_dimension: wgpu::TextureViewDimension::D2,
                                    sample_type: F::FORMAT.sample_type(None, None).unwrap(),
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 1,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                                count: None,
                            },
                        ],
                    })
            }),
            _ => unimplemented!(),
        }
    }

    pub fn texbundle_bind_group<F: texture::TexBundleFormat>(
        &self,
        texbundle: &TextureBundle<F>,
    ) -> wgpu::BindGroup {
        let layout = self.texbundle_bind_group_layout::<F>();
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture bundle"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texbundle.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(texbundle.sampler()),
                },
            ],
        })
    }
}
