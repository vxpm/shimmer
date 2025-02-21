//! POD items that are passed to the GPU for rasterization.

use super::dirty::Region;
use crate::vram::{VRAM_HEIGHT, VRAM_WIDTH};
use encase::{ShaderType, StorageBuffer, internal::WriteInto};
use glam::{IVec2, UVec2, UVec4};
use shimmer::{core::gpu::texture::Depth as TexDepth, gpu::interface};

pub fn to_buffer<T: ShaderType + WriteInto>(value: &T) -> Vec<u8> {
    let mut data = StorageBuffer::new(Vec::new());
    data.write(value).unwrap();
    data.into_inner()
}

#[derive(Debug, Clone, ShaderType)]
pub struct Config {
    pub drawing_area_coords: UVec2,
    pub drawing_area_dimensions: UVec2,

    pub blending_mode: u32,
}

#[derive(Debug, Clone, ShaderType)]
pub struct Vertex {
    pub coords: IVec2,
    pub rgb: UVec4,
    pub uv: UVec2,
}

impl Vertex {
    /// Sorts an slice of vertices in counter-clockwise order.
    pub fn sort(vertices: &mut [Self]) {
        let fold = vertices.iter().fold(IVec2::ZERO, |acc, v| acc + v.coords);
        let center = fold / (vertices.len() as i32);

        vertices.sort_by_key(|v| {
            let relative = v.coords - center;
            let x = relative.x as f32;
            let y = relative.y as f32;

            ordered_float::OrderedFloat(y.atan2(x))
        });
    }
}

#[derive(Debug, Clone, ShaderType, Default)]
pub struct TexConfig {
    mode: u32,
    clut: UVec2,
    texpage: UVec2,
    texwindow_mask: UVec2,
    texwindow_offset: UVec2,
}

impl TexConfig {
    pub fn new(texconfig: interface::TexConfig) -> Self {
        Self {
            mode: match texconfig.texpage.depth() {
                TexDepth::Nibble => 1,
                TexDepth::Byte => 2,
                TexDepth::Full | TexDepth::Reserved => 3,
            },
            clut: UVec2::new(
                u32::from(texconfig.clut.x_by_16().value()) * 16,
                u32::from(texconfig.clut.y().value()),
            ),
            texpage: UVec2::new(
                u32::from(texconfig.texpage.x_base().value()) * 64,
                u32::from(texconfig.texpage.y_base().value()) * 256,
            ),
            texwindow_mask: UVec2::new(
                u32::from(texconfig.texwindow.mask_x().value()),
                u32::from(texconfig.texwindow.mask_y().value()),
            ),
            texwindow_offset: UVec2::new(
                u32::from(texconfig.texwindow.offset_x().value()),
                u32::from(texconfig.texwindow.offset_y().value()),
            ),
        }
    }

    pub fn sampling_region(&self) -> Option<Region> {
        (self.mode != 0)
            .then(|| Region::new((self.texpage.x as u16, self.texpage.y as u16), (64, 256)))
    }
}

#[derive(Debug, Clone, ShaderType)]
pub struct Triangle {
    vertices: [Vertex; 3],
    shading_mode: u32,
    transparency_mode: u32,
    texconfig: TexConfig,
}

impl Triangle {
    pub fn new(triangle: interface::primitive::Triangle) -> Self {
        let texconfig = triangle.texconfig.map(TexConfig::new).unwrap_or_default();

        let mut result = Self {
            vertices: triangle.vertices.map(|v| Vertex {
                coords: IVec2::new(i32::from(v.x.value()), i32::from(v.y.value())),
                rgb: UVec4::new(
                    u32::from(v.color.r),
                    u32::from(v.color.g),
                    u32::from(v.color.b),
                    0,
                ),
                uv: UVec2::new(u32::from(v.u), u32::from(v.v)),
            }),
            shading_mode: triangle.shading as u32,
            transparency_mode: triangle.transparency as u32,
            texconfig,
        };

        Vertex::sort(&mut result.vertices);
        result
    }

    pub fn bounding_region(&self) -> Region {
        let mut min_x = u16::MAX;
        let mut max_x = u16::MIN;
        let mut min_y = u16::MAX;
        let mut max_y = u16::MIN;

        for vertex in &self.vertices {
            let coords = vertex.coords;
            min_x = min_x.min(coords.x.clamp(0, i32::from(VRAM_WIDTH)) as u16);
            max_x = max_x.max(coords.x.clamp(0, i32::from(VRAM_WIDTH)) as u16);

            min_y = min_y.min(coords.y.clamp(0, i32::from(VRAM_HEIGHT)) as u16);
            max_y = max_y.max(coords.y.clamp(0, i32::from(VRAM_HEIGHT)) as u16);
        }

        Region::from_extremes((min_x, min_y), (max_x, max_y))
    }

    pub fn texconfig(&self) -> &TexConfig {
        &self.texconfig
    }
}

#[derive(Debug, Clone, ShaderType)]
pub struct Rectangle {
    top_left: Vertex,
    dimensions: UVec2,
    transparency_mode: u32,
    texconfig: TexConfig,
}

impl Rectangle {
    pub fn new(rectangle: interface::primitive::Rectangle) -> Self {
        let texconfig = rectangle.texconfig.map(TexConfig::new).unwrap_or_default();

        Self {
            top_left: Vertex {
                coords: IVec2::new(
                    i32::from(rectangle.top_left.x.value()),
                    i32::from(rectangle.top_left.y.value()),
                ),
                rgb: UVec4::new(
                    u32::from(rectangle.top_left.color.r),
                    u32::from(rectangle.top_left.color.g),
                    u32::from(rectangle.top_left.color.b),
                    0,
                ),
                uv: UVec2::new(
                    u32::from(rectangle.top_left.u),
                    u32::from(rectangle.top_left.v),
                ),
            },
            dimensions: UVec2::new(u32::from(rectangle.width), u32::from(rectangle.height)),
            transparency_mode: rectangle.transparency as u32,
            texconfig,
        }
    }

    pub fn bounding_region(&self) -> Region {
        Region::new(
            (
                self.top_left.coords.x.clamp(0, i32::from(VRAM_WIDTH)) as u16,
                self.top_left.coords.y.clamp(0, i32::from(VRAM_WIDTH)) as u16,
            ),
            (self.dimensions.x as u16, self.dimensions.y as u16),
        )
    }

    pub fn texconfig(&self) -> &TexConfig {
        &self.texconfig
    }
}
