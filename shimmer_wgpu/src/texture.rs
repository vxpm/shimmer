use std::sync::Arc;
use wgpu::util::DeviceExt;

#[derive(Debug)]
struct TextureBundleViewInner {
    sample_type: wgpu::TextureSampleType,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

#[derive(Debug, Clone)]
pub struct TextureBundleView(Arc<TextureBundleViewInner>);

impl TextureBundleView {
    /// Returns a bind group layout for [`TextureBundleView`]s.
    pub fn bind_group_layout(
        device: &wgpu::Device,
        sample_type: wgpu::TextureSampleType,
    ) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture bundle view"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type,
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
    }

    pub fn bind_group(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture bind group"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.0.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.0.sampler),
                },
            ],
        })
    }

    pub fn sample_type(&self) -> wgpu::TextureSampleType {
        self.0.sample_type
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.0.view
    }
}

pub struct TextureBundle {
    texture: wgpu::Texture,
    view: TextureBundleView,
}

impl TextureBundle {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        descriptor: &wgpu::TextureDescriptor,
        data: &[u8],
    ) -> Self {
        let texture = device.create_texture_with_data(queue, descriptor, Default::default(), data);
        let view = texture.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: descriptor.label.clone(),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let sample_type = descriptor.format.sample_type(None, None).unwrap();
        Self {
            texture,
            view: TextureBundleView(Arc::new(TextureBundleViewInner {
                sample_type,
                view,
                sampler,
            })),
        }
    }

    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn view(&self) -> &TextureBundleView {
        &self.view
    }
}
