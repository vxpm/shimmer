use crate::gpu::{DisplayDepth, DmaDirection, HorizontalResolution, VerticalResolution, VideoMode};
use bitos::{
    bitos,
    integer::{u9, u10, u12},
};

/// A display enable instruction.
#[bitos(32)]
#[derive(Debug)]
pub struct DisplayEnableInstr {
    #[bits(0)]
    pub disabled: bool,
}

/// A DMA direction instruction.
#[bitos(32)]
#[derive(Debug)]
pub struct DmaDirectionInstr {
    #[bits(0..2)]
    pub direction: DmaDirection,
}

/// A display area instruction.
#[bitos(32)]
#[derive(Debug)]
pub struct DisplayAreaInstr {
    #[bits(0..10)]
    pub x: u10,
    #[bits(10..19)]
    pub y: u9,
}

/// A horizontal display range instruction.
#[bitos(32)]
#[derive(Debug)]
pub struct HorizontalDisplayRangeInstr {
    #[bits(0..12)]
    pub x1: u12,
    #[bits(12..24)]
    pub x2: u12,
}

/// A horizontal display range instruction.
#[bitos(32)]
#[derive(Debug)]
pub struct VerticalDisplayRangeInstr {
    #[bits(0..10)]
    pub y1: u10,
    #[bits(10..20)]
    pub y2: u10,
}

/// A horizontal display range instruction.
#[bitos(32)]
#[derive(Debug)]
pub struct DisplayModeInstr {
    #[bits(0..2)]
    pub horizontal_resolution: HorizontalResolution,
    #[bits(2)]
    pub vertical_resolution: VerticalResolution,
    #[bits(3)]
    pub video_mode: VideoMode,
    #[bits(4)]
    pub display_depth: DisplayDepth,
    #[bits(5)]
    pub vertical_interlace: bool,
    #[bits(6)]
    pub force_horizontal_368: bool,
    #[bits(7)]
    pub flip_screen_x: bool,
}
