use bitos::{bitos, integer::u3};
use std::{collections::VecDeque, fmt::Display};
use strum::FromRepr;
use tinylog::{Logger, trace};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reg {
    Reg0,
    Reg1,
    Reg2,
    Reg3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    UnusedA,

    Nop,
    SetLocation,
    Play,
    Forward,
    Backward,
    ReadN,
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
            0x06 => Self::ReadN,
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

#[bitos(8)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Status {
    #[bits(0)]
    pub error: bool,
    #[bits(1)]
    pub motor_on: bool,
    #[bits(2)]
    pub seek_error: bool,
    #[bits(3)]
    pub id_error: bool,
    #[bits(4)]
    pub shell_open: bool,
    #[bits(5)]
    pub read: bool,
    #[bits(6)]
    pub seek: bool,
    #[bits(7)]
    pub play: bool,
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
pub struct CommandStatus {
    /// Current register bank.
    #[bits(0..2)]
    pub bank: Bank,
    /// Is ADPCM busy playing XA-ADPCM?
    #[bits(2)]
    pub adpcm_busy: bool,
    #[bits(3)]
    pub parameter_fifo_empty: bool,
    #[bits(4)]
    pub parameter_fifo_not_full: bool,
    #[bits(5)]
    pub result_fifo_not_empty: bool,
    #[bits(6)]
    pub data_request: bool,
    /// Is the controller busy acknowledging a command?
    #[bits(7)]
    pub busy: bool,
}

#[bitos(3)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
#[derive(Debug, Clone, Copy)]
pub struct InterruptStatus {
    #[bits(0..3)]
    pub kind: InterruptKind,
    #[bits(3)]
    pub sound_buffer_empty: bool,
    #[bits(4)]
    pub sound_buffer_write_ready: bool,
}

impl Default for InterruptStatus {
    fn default() -> Self {
        Self::from_bits(0)
            .with_sound_buffer_empty(true)
            .with_sound_buffer_write_ready(true)
    }
}

#[bitos(8)]
#[derive(Debug, Clone, Copy)]
pub struct InterruptMask {
    #[bits(0..3)]
    pub mask: u3,
    #[bits(3)]
    pub enable_sound_buffer_empty: bool,
    #[bits(4)]
    pub enable_sound_buffer_write_ready: bool,
}

impl Default for InterruptMask {
    fn default() -> Self {
        Self::from_bits(0).with_mask(u3::new(0x7))
    }
}

#[bitos(1)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SectorSize {
    #[default]
    DataOnly,
    Whole,
}

impl SectorSize {
    pub fn value(self) -> usize {
        match self {
            SectorSize::DataOnly => 0x800,
            SectorSize::Whole => 0x924,
        }
    }

    pub fn offset(self) -> usize {
        match self {
            SectorSize::DataOnly => 0x18,
            SectorSize::Whole => 0xC,
        }
    }
}

#[bitos(1)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Speed {
    #[default]
    Normal,
    Double,
}

impl Speed {
    pub fn factor(self) -> u64 {
        match self {
            Speed::Normal => 1,
            Speed::Double => 2,
        }
    }
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

#[derive(Debug, Clone, Copy, Default)]
pub struct Sector {
    minutes: u8,
    seconds: u8,
    frames: u8,
}

impl Sector {
    pub fn new(minutes: u8, seconds: u8, frames: u8) -> Self {
        assert!(seconds < 60);
        assert!(frames < 75);
        Self {
            minutes,
            seconds,
            frames,
        }
    }

    pub fn index(&self) -> Option<u64> {
        let seconds = self.seconds.checked_sub(2);
        seconds.map(|seconds| {
            u64::from(self.minutes) * 60 * 75 + u64::from(seconds) * 75 + u64::from(self.frames)
        })
    }

    pub fn advance(&mut self) {
        self.frames += 1;

        if self.frames == 75 {
            self.frames = 0;
            self.seconds += 1;
        }

        if self.seconds == 60 {
            self.seconds = 0;
            self.minutes += 1;
        }
    }

    pub fn minutes(&self) -> u8 {
        self.minutes
    }

    pub fn seconds(&self) -> u8 {
        self.seconds
    }

    pub fn frames(&self) -> u8 {
        self.frames
    }
}

impl Display for Sector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.minutes, self.seconds, self.frames)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegWrite {
    pub reg: Reg,
    pub value: u8,
}

/// The state of the CDROM controller.
#[derive(Debug)]
pub struct Cdrom {
    pub status: Status,
    pub command_status: CommandStatus,
    pub interrupt_status: InterruptStatus,
    pub interrupt_mask: InterruptMask,
    pub mode: Mode,

    pub location: Sector,
    pub lock_data_queue: bool,

    pub write_queue: VecDeque<RegWrite>,
    pub parameter_queue: VecDeque<u8>,
    pub result_queue: VecDeque<u8>,
    pub sector_data: VecDeque<u8>,

    pub logger: Logger,
}

impl Cdrom {
    pub fn new(logger: Logger) -> Self {
        Self {
            status: Status::default().with_shell_open(true).with_motor_on(true),
            command_status: Default::default(),
            interrupt_status: Default::default(),
            interrupt_mask: Default::default(),
            mode: Default::default(),

            location: Default::default(),
            lock_data_queue: true,

            write_queue: Default::default(),
            parameter_queue: Default::default(),
            result_queue: Default::default(),
            sector_data: Default::default(),

            logger,
        }
    }

    pub fn set_interrupt_kind(&mut self, kind: InterruptKind) {
        self.interrupt_status.set_kind(kind);
    }

    pub fn update_status(&mut self) {
        self.command_status
            .set_parameter_fifo_empty(self.parameter_queue.is_empty());
        self.command_status.set_parameter_fifo_not_full(true);
        self.command_status
            .set_result_fifo_not_empty(!self.result_queue.is_empty());
        self.command_status
            .set_data_request(!self.sector_data.is_empty());
    }

    pub fn read(&mut self, reg: Reg) -> u8 {
        self.update_status();

        match (reg, self.command_status.bank()) {
            (Reg::Reg0, _) => {
                trace!(self.logger, "reading command status"; status = self.command_status);
                self.command_status.to_bits()
            }
            (Reg::Reg1, _) => {
                let value = self.result_queue.pop_front().unwrap();

                trace!(self.logger, "reading result from queue: {value:#02X}");
                value
            }
            (Reg::Reg2, _) => todo!(),
            (Reg::Reg3, Bank::Bank0 | Bank::Bank2) => {
                trace!(
                    self.logger,
                    "reading interrupt mask";
                    mask = self.interrupt_mask
                );
                self.interrupt_mask.to_bits()
            }
            (Reg::Reg3, Bank::Bank1 | Bank::Bank3) => {
                trace!(
                    self.logger,
                    "reading interrupt status";
                    status = self.interrupt_status
                );
                self.interrupt_status.to_bits()
            }
        }
    }

    pub fn read_from_sector(&mut self) -> u8 {
        if self.lock_data_queue {
            return 0;
        }

        self.sector_data.pop_front().unwrap_or_default()
    }
}
