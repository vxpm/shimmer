//! Items related to the GPU of the PSX.

pub mod cmd;
pub mod renderer;
pub mod texture;

mod interpreter;

use crate::cpu;
use bitos::{
    bitos,
    integer::{u1, u4, u9, u10, u12},
};
use std::{collections::VecDeque, ops::Range};
use texture::{TexPage, TransparencyMode};

pub use interpreter::Interpreter;

#[bitos(2)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalResolution {
    R256,
    R320,
    R512,
    R640,
}

impl HorizontalResolution {
    pub fn value(&self) -> u16 {
        match self {
            Self::R256 => 256,
            Self::R320 => 320,
            Self::R512 => 512,
            Self::R640 => 640,
        }
    }
}

#[bitos(1)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalResolution {
    R240,
    R480,
}

impl VerticalResolution {
    pub fn value(&self) -> u16 {
        match self {
            Self::R240 => 240,
            Self::R480 => 480,
        }
    }
}

#[bitos(1)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoMode {
    /// 60Hz
    NTSC,
    /// 50Hz
    PAL,
}

#[bitos(1)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayDepth {
    /// 15 Bit
    Limited,
    /// 24 Bit
    Full,
}

/// The compression mode of colors.
#[bitos(1)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionMode {
    /// Strip LSBs.
    Strip,
    /// Perform dithering.
    Dither,
}

#[bitos(2)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaDirection {
    Off,
    Fifo,
    CpuToGp0,
    GpuToCpu,
}

/// The status of the GPU.
#[bitos(32)]
#[derive(Debug, Clone, Copy)]
pub struct Status {
    #[bits(0..4)]
    pub texpage_x_base: u4,
    #[bits(4..5)]
    pub texpage_y_base: u1,
    #[bits(5..7)]
    pub transparency_mode: TransparencyMode,
    #[bits(7..9)]
    pub texpage_depth: texture::Depth,
    #[bits(9)]
    pub compression_mode: CompressionMode,
    #[bits(10)]
    pub enable_drawing_to_display: bool,
    /// If enabled, drawing sets the mask bit on pixels.
    #[bits(11)]
    pub write_to_mask: bool,
    /// If enabled, pixels can only be drawn to non-masked areas.
    #[bits(12)]
    pub enable_mask: bool,
    #[bits(13)]
    pub interlace: bool,
    #[bits(14)]
    pub flip_screen_x: bool,
    #[bits(15)]
    pub texture_disable: bool,
    #[bits(16..18)]
    pub horizontal_resolution: HorizontalResolution,
    #[bits(18)]
    pub force_horizontal_368: bool,
    #[bits(19)]
    pub vertical_resolution: VerticalResolution,
    #[bits(20)]
    pub video_mode: VideoMode,
    #[bits(21)]
    pub display_depth: DisplayDepth,
    #[bits(22)]
    pub vertical_interlace: bool,
    #[bits(23)]
    pub disable_display: bool,
    #[bits(24)]
    pub interrupt_request: bool,
    #[bits(25)]
    pub dma_request: bool,
    #[bits(26)]
    pub ready_to_receive_cmd: bool,
    #[bits(27)]
    pub ready_to_send_vram: bool,
    #[bits(28)]
    pub ready_to_receive_block: bool,
    #[bits(29..31)]
    pub dma_direction: DmaDirection,
    #[bits(31)]
    pub interlace_odd: bool,
}

impl Default for Status {
    fn default() -> Self {
        Self::from_bits(0x1480_2000)
    }
}

impl Status {
    pub fn update_dreq(&mut self) {
        let dir = self.dma_direction();
        match dir {
            DmaDirection::Off => self.set_dma_request(true),
            DmaDirection::Fifo => self.set_dma_request(true),
            DmaDirection::CpuToGp0 => self.set_dma_request(self.ready_to_receive_block()),
            DmaDirection::GpuToCpu => self.set_dma_request(self.ready_to_send_vram()),
        };
    }

    pub fn texpage(&self) -> TexPage {
        TexPage::default()
            .with_x_base(self.texpage_x_base())
            .with_y_base(self.texpage_y_base())
            .with_transparency_mode(self.transparency_mode())
            .with_depth(self.texpage_depth())
    }
}

/// Environment configuration of the GPU.
#[derive(Debug, Default)]
pub struct EnvironmentState {
    pub double_vram: bool,

    pub textured_rect_flip_x: bool,
    pub textured_rect_flip_y: bool,
}

/// Display configuration of the GPU.
#[derive(Debug, Default)]
pub struct DisplayState {
    pub top_left_x: u10,
    pub top_left_y: u9,

    pub horizontal_range: Range<u12>,
    pub vertical_range: Range<u10>,
}

/// The state of the GPU.
#[derive(Debug, Default)]
pub struct Gpu {
    /// GPU status. This is the value of GPUSTAT (GP0).
    pub status: Status,
    /// GPU response. This is the value of GPUREAD (GP1).
    pub response_queue: VecDeque<u32>,
    /// The queued packets written to GP0.
    pub render_queue: VecDeque<u32>,
    /// The queued packets written to GP1.
    pub display_queue: VecDeque<u32>,

    /// Environment configuration.
    pub environment: EnvironmentState,
    /// Display configuration.
    pub display: DisplayState,
}

impl Gpu {
    #[inline]
    pub fn cycles_per_vblank(&self) -> u32 {
        match self.status.video_mode() {
            VideoMode::NTSC => (f64::from(cpu::FREQUENCY) / 59.826) as u32,
            VideoMode::PAL => (f64::from(cpu::FREQUENCY) / 50.219) as u32,
        }
    }
}
