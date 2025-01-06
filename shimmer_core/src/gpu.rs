pub mod instr;
pub mod software;

use crate::cpu;
use bitos::{
    bitos,
    integer::{u1, u4},
};
use instr::{
    DisplayInstruction, RenderingInstruction,
    environment::{CompressionMode, SemiTransparencyMode, TexturePageDepth},
};

#[bitos(2)]
#[derive(Debug)]
pub enum HorizontalResolution {
    R256 = 0,
    R320 = 1,
    R512 = 2,
    R640 = 3,
}

#[bitos(1)]
#[derive(Debug)]
pub enum VerticalResolution {
    R240 = 0,
    R480 = 1,
}

#[bitos(1)]
#[derive(Debug)]
pub enum VideoMode {
    /// 60Hz
    NTSC = 0,
    /// 50Hz
    PAL = 1,
}

#[bitos(1)]
#[derive(Debug)]
pub enum DisplayDepth {
    /// 15 Bit
    Limited = 0,
    /// 24 Bit
    Full = 1,
}

#[bitos(2)]
#[derive(Debug, PartialEq, Eq)]
pub enum DmaDirection {
    Off = 0,
    Fifo = 1,
    CpuToGp0 = 2,
    GpuToCpu = 3,
}

#[bitos(32)]
#[derive(Debug, Clone, Copy)]
pub struct GpuStatus {
    #[bits(0..4)]
    pub texpage_x_base: u4,
    #[bits(4..5)]
    pub texpage_y_base: u1,
    #[bits(5..7)]
    pub semi_transparency_mode: SemiTransparencyMode,
    #[bits(7..9)]
    pub texpage_depth: Option<TexturePageDepth>,
    #[bits(9..10)]
    pub compression_mode: CompressionMode,
    #[bits(10..11)]
    pub enable_drawing_to_display: bool,
    /// If enabled, drawing sets the mask bit on pixels.
    #[bits(11..12)]
    pub write_to_mask: bool,
    /// If enabled, pixels can only be drawn to non-masked areas.
    #[bits(12..13)]
    pub enable_mask: bool,
    #[bits(13..14)]
    pub interlace: bool,
    #[bits(14..15)]
    pub flip_screen_x: bool,
    #[bits(16..18)]
    pub horizontal_resolution: HorizontalResolution,
    #[bits(18..19)]
    pub force_horizontal_368: bool,
    #[bits(19..20)]
    pub vertical_resolution: VerticalResolution,
    #[bits(20..21)]
    pub video_mode: VideoMode,
    #[bits(21..22)]
    pub display_depth: DisplayDepth,
    #[bits(22..23)]
    pub vertical_interlace: bool,
    #[bits(23..24)]
    pub disable_display: bool,
    #[bits(24..25)]
    pub interrupt_request: bool,
    #[bits(25..26)]
    pub dma_request: bool,
    #[bits(26..27)]
    pub ready_to_receive_packet: bool,
    #[bits(27..28)]
    pub ready_to_send_vram: bool,
    #[bits(28..29)]
    pub ready_to_receive_block: bool,
    #[bits(29..31)]
    pub dma_direction: DmaDirection,
    #[bits(31..32)]
    pub interlace_odd: bool,
}

impl Default for GpuStatus {
    fn default() -> Self {
        Self::from_bits(0x1480_2000)
    }
}

#[derive(Default)]
pub struct State {
    pub status: GpuStatus,
    pub queue: Vec<RenderingInstruction>,
    pub display_queue: Vec<DisplayInstruction>,
}

impl State {
    #[inline]
    pub fn cycles_per_vblank(&self) -> u32 {
        match self.status.video_mode() {
            VideoMode::NTSC => (cpu::FREQUENCY as f64 / 59.826) as u32,
            VideoMode::PAL => (cpu::FREQUENCY as f64 / 50.219) as u32,
        }
    }
}
