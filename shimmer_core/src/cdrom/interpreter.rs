mod command;
mod control;
mod interrupt;

use std::{
    collections::VecDeque,
    io::{Read, Seek},
};

use super::{Bank, Event, InterruptKind, Mode, Reg, RegWrite};
use crate::{PSX, interrupts::Interrupt, scheduler};
use tinylog::{debug, info, trace, warn};

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
            debug!(psx.loggers.cdrom, "popped interrupt: {:?}", kind);
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
            Event::Acknowledge => {
                info!(psx.loggers.cdrom, "acknowledged command");
                psx.cdrom.command_status.set_busy(false);
                psx.cdrom.result_queue.push_back(psx.cdrom.status.to_bits());
                self.interrupt_queue.push_back(InterruptKind::Acknowledge);
            }
            Event::CompleteInit => {
                info!(psx.loggers.cdrom, "init complete");
                psx.cdrom.mode = Mode::from_bits(0x20);
                psx.cdrom.result_queue.push_back(psx.cdrom.status.to_bits());
                self.interrupt_queue.push_back(InterruptKind::Complete);
            }
            Event::CompleteGetID => {
                debug!(psx.loggers.cdrom, "get id complete");
                psx.cdrom
                    .result_queue
                    .extend([0x02, 0x00, 0x20, 0x00, 0x53, 0x43, 0x45, 0x42]);
                self.interrupt_queue.push_back(InterruptKind::Complete);
            }
            Event::Read(first) => {
                if first {
                    psx.cdrom.status.set_read(true);
                }

                let mut rom = psx.cdrom.rom.as_ref().unwrap();
                let size = psx.cdrom.mode.sector_size().value();

                let mut buf = vec![0; size];
                let byte_index = psx.cdrom.location.0 * 0x930;
                let offset = match psx.cdrom.mode.sector_size() {
                    super::SectorSize::DataOnly => 0x18,
                    super::SectorSize::Whole => 0x0C,
                };

                info!(
                    psx.loggers.cdrom,
                    "read from sector {}", psx.cdrom.location.0
                );

                rom.seek(std::io::SeekFrom::Start(byte_index + offset as u64))
                    .unwrap();
                rom.read_exact(&mut buf).unwrap();
                psx.cdrom.data_queue.extend(buf);

                psx.cdrom.result_queue.push_back(psx.cdrom.status.to_bits());
                self.interrupt_queue.push_back(InterruptKind::DataReady);

                if psx.cdrom.status.read() {
                    psx.cdrom.location.0 += 1;
                    psx.scheduler.schedule(
                        scheduler::Event::Cdrom(Event::Read(false)),
                        command::READ_DELAY / psx.cdrom.mode.speed().factor(),
                    );
                }
            }
            Event::CompletePause => {
                info!(psx.loggers.cdrom, "pause complete");
                psx.cdrom.status.set_read(false);
                psx.cdrom.result_queue.push_back(psx.cdrom.status.to_bits());
                self.interrupt_queue.push_back(InterruptKind::Complete);
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
