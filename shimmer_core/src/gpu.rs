pub mod instr;

use bitos::{
    bitos,
    integer::{u1, u4},
};
use instr::environment::{CompressionMode, SemiTransparencyMode, TexturePageDepth};

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
    texpage_x_base: u4,
    #[bits(4..5)]
    texpage_y_base: u1,
    #[bits(5..7)]
    semi_transparency_mode: SemiTransparencyMode,
    #[bits(7..9)]
    texpage_depth: Option<TexturePageDepth>,
    #[bits(9..10)]
    compression_mode: CompressionMode,
    #[bits(10..11)]
    enable_drawing_to_display: bool,
    /// If enabled, drawing sets the mask bit on pixels.
    #[bits(11..12)]
    write_to_mask: bool,
    /// If enabled, pixels can only be drawn to non-masked areas.
    #[bits(12..13)]
    enable_mask: bool,
    #[bits(13..14)]
    interlace: bool,
    #[bits(14..15)]
    flip_screen_x: bool,
    #[bits(16..18)]
    horizontal_resolution: HorizontalResolution,
    #[bits(18..19)]
    force_horizontal_368: bool,
    #[bits(19..20)]
    vertical_resolution: VerticalResolution,
    #[bits(20..21)]
    video_mode: VideoMode,
    #[bits(21..22)]
    display_depth: DisplayDepth,
    #[bits(22..23)]
    vertical_interlace: bool,
    #[bits(23..24)]
    disable_display: bool,
    #[bits(24..25)]
    interrupt_request: bool,
    #[bits(25..26)]
    dma_request: bool,
    #[bits(26..27)]
    ready_to_receive_packet: bool,
    #[bits(27..28)]
    ready_to_send_vram: bool,
    #[bits(28..29)]
    ready_to_receive_block: bool,
    #[bits(29..31)]
    dma_direction: DmaDirection,
    #[bits(31..32)]
    interlace_odd: bool,
}

impl Default for GpuStatus {
    fn default() -> Self {
        Self::from_bits(0x1480_2000)
    }
}

#[derive(Default)]
pub struct State {
    pub status: GpuStatus,
}

impl State {
    #[inline]
    pub fn cycles_per_vblank(&self) -> u32 {
        match self.status.video_mode() {
            VideoMode::NTSC => 33_870_000 / 60,
            VideoMode::PAL => 33_870_000 / 50,
        }
    }
}
