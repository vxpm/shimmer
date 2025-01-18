use std::{
    marker::PhantomData,
    sync::{Arc, OnceLock},
};

pub trait TexBundleFormat: std::fmt::Debug + Clone + Copy + Default {
    const FORMAT: wgpu::TextureFormat;
}

#[derive(Debug)]
pub(super) struct TextureBundleInner<S> {
    pub(super) texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) sampler: wgpu::Sampler,
    pub(super) _phantom: PhantomData<S>,
}

#[derive(Debug, Clone)]
pub struct TextureBundle<S>(pub(super) Arc<TextureBundleInner<S>>);

impl<S> TextureBundle<S> {
    pub fn texture(&self) -> &wgpu::Texture {
        &self.0.texture
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.0.view
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.0.sampler
    }
}

#[derive(Default)]
pub(super) struct TexBundleBgLayouts {
    pub(super) r16uint: OnceLock<wgpu::BindGroupLayout>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct R16Uint;

impl TexBundleFormat for R16Uint {
    const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R16Uint;
}
