mod command;
mod control;
mod interrupt;

use std::collections::VecDeque;

use super::{Bank, Event, InterruptKind, Mode, Reg, RegWrite};
use crate::{PSX, interrupts::Interrupt, scheduler};
use tinylog::{debug, trace, warn};

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
                psx.cdrom.command_status.set_busy(false);
                psx.cdrom.result_queue.push_back(psx.cdrom.status.to_bits());
                self.interrupt_queue.push_back(InterruptKind::Acknowledge);
            }
            Event::CompleteInit => {
                debug!(psx.loggers.cdrom, "INIT complete");
                psx.cdrom.mode = Mode::from_bits(0x20);
                psx.cdrom.result_queue.push_back(psx.cdrom.status.to_bits());
                self.interrupt_queue.push_back(InterruptKind::Complete);
            }
            Event::CompleteGetID => {
                debug!(psx.loggers.cdrom, "GetID complete");
                psx.cdrom
                    .result_queue
                    .extend([0x02, 0x00, 0x20, 0x00, 0x53, 0x43, 0x45, 0x42]);
                self.interrupt_queue.push_back(InterruptKind::Complete);
            }
            Event::Read => {
                psx.cdrom.command_status.set_data_request(true);
                psx.cdrom.result_queue.push_back(0);
                self.interrupt_queue.push_back(InterruptKind::DataReady);

                psx.scheduler.schedule(
                    scheduler::Event::Cdrom(Event::Read),
                    2 * command::DEFAULT_DELAY,
                );
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
