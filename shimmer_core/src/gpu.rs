pub mod commands;
pub mod interpreter;

pub use interpreter::Interpreter;

use crate::cpu;
use bitos::{
    bitos,
    integer::{u1, u4, u9, u10, u12},
};
use commands::{
    Packet,
    environment::{CompressionMode, SemiTransparencyMode, TexturePageDepth},
    rendering::{CoordPacket, SizePacket},
};
use std::{collections::VecDeque, ops::Range};

#[bitos(2)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalResolution {
    R256 = 0,
    R320 = 1,
    R512 = 2,
    R640 = 3,
}

#[bitos(1)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalResolution {
    R240 = 0,
    R480 = 1,
}

#[bitos(1)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoMode {
    /// 60Hz
    NTSC = 0,
    /// 50Hz
    PAL = 1,
}

#[bitos(1)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayDepth {
    /// 15 Bit
    Limited = 0,
    /// 24 Bit
    Full = 1,
}

#[bitos(2)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub texpage_depth: TexturePageDepth,
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
    #[bits(15)]
    pub texpage_y_base_2: u1,
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
        Self::from_bits(0x1480_2000).with_ready_to_send_vram(true)
    }
}

#[bitos(32)]
#[derive(Debug, Clone, Copy, Default)]
pub struct GpuResponse {}

#[derive(Debug, Default)]
pub struct EnvironmentState {
    pub textured_rect_flip_x: bool,
    pub textured_rect_flip_y: bool,
}

#[derive(Debug, Default)]
pub struct DisplayState {
    pub area_start_x: u10,
    pub area_start_y: u9,

    pub horizontal_range: Range<u12>,
    pub vertical_range: Range<u10>,
}

#[derive(Debug, Default)]
pub enum ExecState {
    /// Currently not executing anything
    #[default]
    None,
    /// Waiting for enough data to complete
    CpuToVramBlit { dest: CoordPacket, size: SizePacket },
}

#[derive(Debug, Default)]
pub struct Queue {
    packets: VecDeque<Packet>,
    render_len: usize,
    display_len: usize,
}

impl Queue {
    pub fn len(&self) -> usize {
        self.packets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn render_len(&self) -> usize {
        self.render_len
    }

    pub fn display_len(&self) -> usize {
        self.display_len
    }

    pub fn enqueue(&mut self, packet: Packet) {
        match packet {
            Packet::Rendering(_) => self.render_len += 1,
            Packet::Display(_) => self.display_len += 1,
        }

        self.packets.push_back(packet);
    }

    pub fn front(&mut self) -> Option<&Packet> {
        self.packets.front()
    }

    pub fn front_render(&mut self) -> Option<u32> {
        let index = self
            .packets
            .iter()
            .position(|p| matches!(p, Packet::Rendering(_)))?;

        self.packets.get(index).map(|p| match p {
            Packet::Rendering(value) => *value,
            _ => unreachable!(),
        })
    }

    pub fn front_display(&mut self) -> Option<u32> {
        let index = self
            .packets
            .iter()
            .position(|p| matches!(p, Packet::Rendering(_)))?;

        self.packets.get(index).map(|p| match p {
            Packet::Display(value) => *value,
            _ => unreachable!(),
        })
    }

    pub fn pop(&mut self) -> Option<Packet> {
        let value = self.packets.pop_front();
        match value {
            Some(Packet::Rendering(_)) => self.render_len -= 1,
            Some(Packet::Display(_)) => self.display_len -= 1,
            _ => (),
        }

        value
    }

    pub fn pop_render(&mut self) -> Option<u32> {
        let index = self
            .packets
            .iter()
            .position(|p| matches!(p, Packet::Rendering(_)))?;

        self.render_len -= 1;
        self.packets.remove(index).map(|p| match p {
            Packet::Rendering(value) => value,
            _ => unreachable!(),
        })
    }

    pub fn pop_display(&mut self) -> Option<u32> {
        let index = self
            .packets
            .iter()
            .position(|p| matches!(p, Packet::Rendering(_)))?;

        self.display_len -= 1;
        self.packets.remove(index).map(|p| match p {
            Packet::Display(value) => value,
            _ => unreachable!(),
        })
    }
}

#[derive(Debug, Default)]
pub struct State {
    pub status: GpuStatus,
    pub response: GpuResponse,
    pub queue: Queue,

    pub environment: EnvironmentState,
    pub display: DisplayState,

    pub execution_state: ExecState,
}

impl State {
    #[inline]
    pub fn cycles_per_vblank(&self) -> u32 {
        match self.status.video_mode() {
            VideoMode::NTSC => (f64::from(cpu::FREQUENCY) / 59.826) as u32,
            VideoMode::PAL => (f64::from(cpu::FREQUENCY) / 50.219) as u32,
        }
    }
}
