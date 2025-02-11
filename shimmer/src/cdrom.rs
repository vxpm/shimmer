mod command;
mod control;
mod interrupt;

use crate::{PSX, scheduler};
use shimmer_core::{
    cdrom::{Bank, Command, InterruptKind, Mode, Reg, RegWrite, Sector},
    interrupts::Interrupt,
};
use std::{
    collections::VecDeque,
    io::{Read, Seek},
};
use tinylog::{debug, info, trace, warn};

pub const CDROM_VERSION: [u8; 4] = [0x94, 0x09, 0x19, 0xc0];

pub const COMPLETE_GETID_DELAY: u64 = 18_944;
pub const COMPLETE_PAUSE_DELAY: u64 = 2_168_860;
pub const COMPLETE_PAUSE_NOP_DELAY: u64 = 7_666;
pub const READ_DELAY: u64 = 451_021;
pub const SEEK_DELAY: u64 = 33_869;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Update,
    Acknowledge(Command),
    Complete(Command),
    Read,
}

#[derive(Debug, Clone, Default)]
pub struct Interpreter {
    command_queue: VecDeque<u8>,
    interrupt_queue: VecDeque<InterruptKind>,
}

impl Interpreter {
    fn next_interrupt(&mut self, psx: &mut PSX) {
        if psx.cdrom.interrupt_status.kind() == InterruptKind::None
            && let Some(kind) = self.interrupt_queue.pop_front()
        {
            psx.cdrom.set_interrupt_kind(kind);
        }
    }

    pub fn update(&mut self, psx: &mut PSX, event: Event) {
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
                            COMPLETE_PAUSE_DELAY / psx.cdrom.mode.speed().factor()
                        } else {
                            COMPLETE_PAUSE_NOP_DELAY
                        };
                        sched_complete(psx, delay);
                    }
                    Command::SeekL => {
                        assert!(!psx.cdrom.status.read());
                        psx.cdrom.status.set_seek(true);
                        sched_complete(psx, SEEK_DELAY);
                    }
                    Command::SetLocation => {
                        let minutes = psx.cdrom.parameter_queue.pop_front().unwrap();
                        let seconds = psx.cdrom.parameter_queue.pop_front().unwrap();
                        let frames = psx.cdrom.parameter_queue.pop_front().unwrap();
                        let decode_bcd = |value| (value & 0x0F) + 10u8 * ((value & 0xF0) >> 4);
                        psx.cdrom.location = Sector::new(
                            decode_bcd(minutes),
                            decode_bcd(seconds) - 2,
                            decode_bcd(frames),
                        );

                        info!(
                            psx.loggers.cdrom,
                            "set location {}:{}:{}",
                            decode_bcd(minutes),
                            decode_bcd(seconds) - 2,
                            decode_bcd(frames); sector = psx.cdrom.location.0
                        );
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
                    _ => todo!("ack {cmd:?}"),
                }

                debug!(psx.loggers.cdrom, "acknowledging {cmd:?}"; stat = psx.cdrom.status);
                if push_stat {
                    psx.cdrom.result_queue.push_back(psx.cdrom.status.to_bits());
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
                self.interrupt_queue.push_back(InterruptKind::Complete);
            }
            Event::Read => {
                if !psx.cdrom.status.read() {
                    return;
                }

                let mut rom = psx.cdrom.rom.as_ref().unwrap();
                let size = psx.cdrom.mode.sector_size().value();
                let offset = psx.cdrom.mode.sector_size().offset();

                info!(
                    psx.loggers.cdrom,
                    "read from sector {}", psx.cdrom.location.0
                );

                let mut buf = vec![0; size];
                let start_byte = psx.cdrom.location.0 * 0x930;
                rom.seek(std::io::SeekFrom::Start(start_byte + offset as u64))
                    .unwrap();
                rom.read_exact(&mut buf).unwrap();

                psx.cdrom.sector_data = VecDeque::from(buf);
                psx.cdrom.location.0 += 1;
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
