mod interpreter;

use bitos::{bitos, integer::u3};
use std::collections::VecDeque;
use strum::FromRepr;

pub use interpreter::Interpreter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    UnusedA,

    Nop,
    SetLocation,
    Play,
    Forward,
    Backward,
    ReadCount,
    Standby,
    Stop,
    Pause,
    Init,
    Mute,
    Demute,
    SetFilter,
    SetMode,
    GetParam,
    GetLocationL,
    GetLocationP,
    SetSession,
    GetTN,
    GetTD,
    SeekL,
    SeekP,

    Test,
    GetID,
    ReadS,
    Reset,
    GetQ,
    ReadTOC,
    VideoCD,

    Unlock0,
    Unlock1,
    Unlock2,
    Unlock3,
    Unlock4,
    Unlock5,
    Unlock6,
    Lock,

    UnusedB,
}

impl Command {
    pub fn new(value: u8) -> Self {
        match value {
            0x00 => Self::UnusedA,
            0x01 => Self::Nop,
            0x02 => Self::SetLocation,
            0x03 => Self::Play,
            0x04 => Self::Forward,
            0x05 => Self::Backward,
            0x06 => Self::ReadCount,
            0x07 => Self::Standby,
            0x08 => Self::Stop,
            0x09 => Self::Pause,
            0x0a => Self::Init,
            0x0b => Self::Mute,
            0x0c => Self::Demute,
            0x0d => Self::SetFilter,
            0x0e => Self::SetMode,
            0x0f => Self::GetParam,
            0x10 => Self::GetLocationL,
            0x11 => Self::GetLocationP,
            0x12 => Self::SetSession,
            0x13 => Self::GetTN,
            0x14 => Self::GetTD,
            0x15 => Self::SeekL,
            0x16 => Self::SeekP,
            0x17 => Self::UnusedA,
            0x18 => Self::UnusedA,
            0x19 => Self::Test,
            0x1a => Self::GetID,
            0x1b => Self::ReadS,
            0x1c => Self::Reset,
            0x1d => Self::GetQ,
            0x1e => Self::ReadTOC,
            0x1f => Self::VideoCD,
            0x20..=0x4f => Self::UnusedA,
            0x50 => Self::Unlock0,
            0x51 => Self::Unlock1,
            0x52 => Self::Unlock2,
            0x53 => Self::Unlock3,
            0x54 => Self::Unlock4,
            0x55 => Self::Unlock5,
            0x56 => Self::Unlock6,
            0x57 => Self::Lock,
            0x58..=0x5f => Self::UnusedB,
            0x60..=0xFF => Self::UnusedA,
        }
    }
}

#[bitos(2)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, FromRepr)]
pub enum Bank {
    #[default]
    Bank0,
    Bank1,
    Bank2,
    Bank3,
}

#[bitos(8)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Status {
    /// Current register bank.
    #[bits(0..2)]
    pub bank: Bank,
    /// Is ADPCM busy playing XA-ADPCM?
    #[bits(2)]
    pub adpcm_busy: bool,
    #[bits(3)]
    pub parameter_fifo_empty: bool,
    #[bits(4)]
    pub parameter_fifo_ready: bool,
    #[bits(5)]
    pub result_fifo_ready: bool,
    #[bits(6)]
    pub data_request: bool,
    /// Is the controller busy acknowledging a command?
    #[bits(7)]
    pub busy: bool,
}

#[bitos(3)]
#[derive(Debug, Clone, Copy, Default)]
pub enum InterruptKind {
    #[default]
    None,
    DataReady,
    Complete,
    Acknowledge,
    DataEnd,
    DiskError,
    Unknown6,
    Unknown7,
}

#[bitos(8)]
#[derive(Debug, Clone, Copy, Default)]
pub struct InterruptStatus {
    #[bits(0..3)]
    kind: InterruptKind,
    #[bits(3)]
    sound_buffer_empty: bool,
    #[bits(4)]
    sound_buffer_write_ready: bool,
}

#[bitos(8)]
#[derive(Debug, Clone, Copy, Default)]
pub struct InterruptMask {
    #[bits(0..3)]
    mask: u3,
    #[bits(3)]
    enable_sound_buffer_empty: bool,
    #[bits(4)]
    enable_sound_buffer_write_ready: bool,
}

#[bitos(1)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SectorSize {
    #[default]
    DataOnly,
    Whole,
}

#[bitos(1)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Speed {
    #[default]
    Normal,
    Double,
}

#[bitos(8)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Mode {
    #[bits(0)]
    pub cdda: bool,
    #[bits(1)]
    pub auto_pause: bool,
    #[bits(2)]
    pub report: bool,
    #[bits(3)]
    pub xa_filter: bool,
    #[bits(4)]
    pub ignore: bool,
    #[bits(5)]
    pub sector_size: SectorSize,
    #[bits(6)]
    pub xa_adpcm: bool,
    #[bits(7)]
    pub speed: Speed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegWrite {
    Reg0(u8),
    Reg1(u8),
    Reg2(u8),
    Reg3(u8),
}

/// The state of the CDROM controller.
#[derive(Debug, Clone, Default)]
pub struct Controller {
    pub status: Status,
    pub interrupt_status: InterruptStatus,
    pub interrupt_mask: InterruptMask,
    pub mode: Mode,

    pub write_queue: VecDeque<RegWrite>,
    pub result_fifo: Vec<u8>,
}

impl Controller {
    pub fn read_reg0(&self) -> u8 {
        self.status.to_bits()
    }

    pub fn read_reg1(&self) -> u8 {
        todo!()
    }

    pub fn read_reg2(&self) -> u8 {
        todo!()
    }

    pub fn read_reg3(&self) -> u8 {
        match self.status.bank() {
            Bank::Bank0 | Bank::Bank2 => todo!(),
            Bank::Bank1 | Bank::Bank3 => self.interrupt_status.to_bits(),
        }
    }

    fn command(&mut self, command: Command) {
        match command {
            Command::UnusedA => {
                self.result_fifo.push(self.status.to_bits());
            }
            _ => todo!("{:?}", command),
        }
    }
}
