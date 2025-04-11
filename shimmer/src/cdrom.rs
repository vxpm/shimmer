mod command;
mod control;
mod interrupt;

use crate::{PSX, scheduler};
use shimmer_core::{
    CYCLES_MICROS, CYCLES_MILLIS, Cycles,
    cdrom::{Bank, Command, InterruptKind, Mode, Reg, RegWrite, Sector},
    interrupts::Interrupt,
};
use std::{
    collections::VecDeque,
    io::{Read, Seek},
};
use tinylog::{debug, error, info, trace, warn};

pub const CDROM_VERSION: [u8; 4] = [0x94, 0x09, 0x19, 0xc0];

pub const COMPLETE_GETID_DELAY: Cycles = 574 * CYCLES_MICROS;
pub const COMPLETE_PAUSE_DELAY: Cycles = 64 * CYCLES_MILLIS + 36 * CYCLES_MICROS;
pub const COMPLETE_PAUSE_NOP_DELAY: Cycles = 232 * CYCLES_MICROS;
pub const READ_DELAY: Cycles = 13 * CYCLES_MILLIS + 325 * CYCLES_MICROS;
pub const SEEK_DELAY: Cycles = 1 * CYCLES_MILLIS;

pub trait Rom: std::fmt::Debug + std::io::Read + std::io::Seek + Send {}
impl<T> Rom for T where T: std::fmt::Debug + std::io::Read + std::io::Seek + Send {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Update,
    Acknowledge(Command),
    Complete(Command),
    Read,
}

#[derive(Debug, Default)]
pub struct Cdrom {
    rom: Option<Box<dyn Rom>>,
    command_queue: VecDeque<u8>,
    interrupt_queue: VecDeque<InterruptKind>,
}

impl Cdrom {
    pub fn new(rom: Option<Box<dyn Rom>>) -> Self {
        Self {
            rom,
            command_queue: VecDeque::new(),
            interrupt_queue: VecDeque::new(),
        }
    }

    fn next_interrupt(&mut self, psx: &mut PSX) {
        if psx.cdrom.interrupt_status.kind() == InterruptKind::None
            && let Some(kind) = self.interrupt_queue.pop_front()
        {
            psx.cdrom.set_interrupt_kind(kind);
        }
    }

    pub fn insert_rom<R>(&mut self, rom: R)
    where
        R: Rom + 'static,
    {
        self.rom = Some(Box::new(rom))
    }

    pub fn remove_rom(&mut self) {
        self.rom = None;
    }

    pub fn update(&mut self, psx: &mut PSX, event: Event) {
        psx.cdrom.status.set_shell_open(self.rom.is_none());
        psx.cdrom.update_status();

        match event {
            Event::Update => {
                while let Some(RegWrite { reg, value }) = psx.cdrom.write_queue.pop_front() {
                    match (reg, psx.cdrom.command_status.bank()) {
                        (Reg::Reg0, _) => {
                            let bank = Bank::from_repr(value as usize & 0b11).unwrap();
                            psx.cdrom.command_status.set_bank(bank);

                            trace!(psx.loggers.cdrom, "switched to {:?}", bank);
                        }

                        (Reg::Reg1, Bank::Bank0) => self.command(psx, value),
                        (Reg::Reg1, Bank::Bank1) => todo!(),
                        (Reg::Reg1, Bank::Bank2) => todo!(),
                        (Reg::Reg1, Bank::Bank3) => warn!(psx.loggers.cdrom, "ignoring ATV2 write"),

                        (Reg::Reg2, Bank::Bank0) => self.push_parameter(psx, value),
                        (Reg::Reg2, Bank::Bank1) => self.set_interrupt_mask(psx, value),
                        (Reg::Reg2, Bank::Bank2) => warn!(psx.loggers.cdrom, "ignoring ATV0 write"),
                        (Reg::Reg2, Bank::Bank3) => warn!(psx.loggers.cdrom, "ignoring ATV3 write"),

                        (Reg::Reg3, Bank::Bank0) => self.control_request(psx, value),
                        (Reg::Reg3, Bank::Bank1) => self.ack_interrupt_status(psx, value),
                        (Reg::Reg3, Bank::Bank2) => warn!(psx.loggers.cdrom, "ignoring ATV1 write"),
                        (Reg::Reg3, Bank::Bank3) => {
                            warn!(psx.loggers.cdrom, "ignoring ADPCTL write")
                        }
                    }
                }
            }
            Event::Acknowledge(cmd) => {
                assert!(psx.cdrom.command_status.busy());
                psx.cdrom.command_status.set_busy(false);

                let sched_complete = |psx: &mut PSX, delay| {
                    psx.scheduler
                        .schedule(scheduler::Event::Cdrom(Event::Complete(cmd)), delay);
                };

                let mut push_stat = true;
                match cmd {
                    Command::Nop | Command::Demute | Command::Mute => (),
                    Command::Init => {
                        sched_complete(psx, READ_DELAY);
                    }
                    Command::Test => {
                        let param = psx.cdrom.parameter_queue.pop_front().unwrap_or_default();
                        if param != 0x20 {
                            todo!("cdrom test command with parameter {param}")
                        }

                        psx.cdrom.result_queue.extend(CDROM_VERSION);
                        push_stat = false;
                    }
                    Command::GetID => sched_complete(psx, COMPLETE_GETID_DELAY),
                    Command::ReadN | Command::ReadS => {
                        assert!(!psx.cdrom.status.seek());
                        psx.cdrom.status.set_read(true);
                        psx.scheduler.schedule(
                            scheduler::Event::Cdrom(Event::Read),
                            READ_DELAY / psx.cdrom.mode.speed().factor(),
                        );
                    }
                    Command::Pause => {
                        let delay = if psx.cdrom.status.read() {
                            COMPLETE_PAUSE_DELAY
                        } else {
                            COMPLETE_PAUSE_NOP_DELAY
                        };
                        sched_complete(psx, delay);
                    }
                    Command::SeekL => {
                        psx.cdrom.status.set_read(false);
                        psx.cdrom.status.set_seek(true);
                        sched_complete(psx, SEEK_DELAY);
                    }
                    Command::SetLocation => {
                        let decode_bcd = |value| (value & 0x0F) + 10u8 * ((value & 0xF0) >> 4);

                        let minutes = decode_bcd(psx.cdrom.parameter_queue.pop_front().unwrap());
                        let seconds = decode_bcd(psx.cdrom.parameter_queue.pop_front().unwrap());
                        let frames = decode_bcd(psx.cdrom.parameter_queue.pop_front().unwrap());

                        psx.cdrom.location = Sector::new(minutes, seconds, frames);

                        info!(psx.loggers.cdrom, "set location {}", psx.cdrom.location);
                    }
                    Command::SetMode => {
                        psx.cdrom.mode =
                            Mode::from_bits(psx.cdrom.parameter_queue.pop_front().unwrap());
                        info!(psx.loggers.cdrom, "set mode"; mode = psx.cdrom.mode);
                    }
                    Command::SetFilter => {
                        let file = psx.cdrom.parameter_queue.pop_front().unwrap();
                        let channel = psx.cdrom.parameter_queue.pop_front().unwrap();
                        info!(psx.loggers.cdrom, "set filter"; file = file, channel = channel);
                    }
                    Command::GetLocationP => {
                        let encode_bcd = |value: u8| 10 * (value / 10) + (value % 10);
                        psx.cdrom.result_queue.extend([
                            0x01,
                            0x01,
                            encode_bcd(psx.cdrom.location.minutes()),
                            encode_bcd(psx.cdrom.location.seconds()),
                            encode_bcd(psx.cdrom.location.frames()),
                            encode_bcd(psx.cdrom.location.minutes()),
                            encode_bcd(psx.cdrom.location.seconds()),
                            encode_bcd(psx.cdrom.location.frames()),
                        ]);

                        info!(psx.loggers.cdrom, "get location");
                    }
                    // TODO: unstub
                    Command::GetTN | Command::GetTD => {
                        psx.cdrom.result_queue.extend([1, 1]);
                    }
                    _ => {
                        error!(
                            psx.loggers.cdrom,
                            "tried to ack {cmd:?} but it has no implementation yet"
                        );
                    }
                }

                debug!(psx.loggers.cdrom, "acknowledging {cmd:?}"; stat = psx.cdrom.status);
                if push_stat {
                    psx.cdrom
                        .result_queue
                        .push_front(psx.cdrom.status.to_bits());
                }
                self.interrupt_queue.push_back(InterruptKind::Acknowledge);
            }
            Event::Complete(cmd) => {
                let mut push_stat = true;
                match cmd {
                    Command::Init => {
                        psx.cdrom.mode = Mode::from_bits(0x20);
                    }
                    Command::GetID => {
                        psx.cdrom
                            .result_queue
                            .extend([0x02, 0x00, 0x20, 0x00, 0x53, 0x43, 0x45, 0x41]);
                        push_stat = false;
                    }
                    Command::Pause => {
                        psx.cdrom.status.set_read(false);
                    }
                    Command::SeekL => {
                        psx.cdrom.status.set_seek(false);
                    }
                    _ => todo!("complete {cmd:?}"),
                }

                debug!(psx.loggers.cdrom, "completing {cmd:?}"; stat = psx.cdrom.status);
                if push_stat {
                    psx.cdrom.result_queue.push_back(psx.cdrom.status.to_bits());
                }
                self.interrupt_queue.push_front(InterruptKind::Complete);
            }
            Event::Read => {
                if !psx.cdrom.status.read() {
                    return;
                }

                let Some(rom) = &mut self.rom else {
                    panic!("reading without a disk");
                };

                info!(psx.loggers.cdrom, "read from sector {}", psx.cdrom.location);
                let size = psx.cdrom.mode.sector_size().value();
                let offset = psx.cdrom.mode.sector_size().offset();

                if let Some(index) = psx.cdrom.location.index() {
                    let mut buf = vec![0; size];
                    let start_byte = index * 0x930;
                    rom.seek(std::io::SeekFrom::Start(start_byte + offset as u64))
                        .unwrap();
                    rom.read_exact(&mut buf).unwrap();

                    psx.cdrom.sector_data = VecDeque::from(buf);
                } else {
                    error!(psx.loggers.cdrom, "reading from pregap");
                    psx.cdrom.sector_data = VecDeque::from(vec![0; size]);
                }

                psx.cdrom.location.advance();
                psx.scheduler.schedule(
                    scheduler::Event::Cdrom(Event::Read),
                    READ_DELAY / psx.cdrom.mode.speed().factor(),
                );

                psx.cdrom.result_queue.push_back(psx.cdrom.status.to_bits());
                self.interrupt_queue.push_back(InterruptKind::DataReady);
            }
        }

        if psx.cdrom.interrupt_status.kind() == InterruptKind::None {
            while let Some(value) = self.command_queue.pop_front() {
                self.command(psx, value);
            }
        }

        self.next_interrupt(psx);

        let masked =
            psx.cdrom.interrupt_status.kind() as u8 & psx.cdrom.interrupt_mask.mask().value();

        if masked != 0 {
            psx.interrupts.status.request(Interrupt::CDROM);
        }

        psx.cdrom.update_status();
    }
}
