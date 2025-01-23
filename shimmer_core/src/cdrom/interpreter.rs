mod command;
mod control;
mod interrupt;

use std::collections::VecDeque;

use super::{Bank, Event, InterruptKind, Mode, Reg, RegWrite};
use crate::{PSX, interrupts::Interrupt};
use tinylog::{debug, trace};

#[derive(Debug, Clone, Default)]
pub struct Interpreter {
    command_queue: VecDeque<u8>,
    interrupt_queue: VecDeque<InterruptKind>,
}

impl Interpreter {
    fn switch_bank(&mut self, psx: &mut PSX, bank: Bank) {
        psx.cdrom.status.set_bank(bank);
    }

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
                    match (reg, psx.cdrom.status.bank()) {
                        (Reg::Reg0, _) => {
                            let bank = Bank::from_repr(value as usize & 0b11).unwrap();
                            self.switch_bank(psx, bank);
                            trace!(psx.loggers.cdrom, "switched to {:?}", bank);
                        }

                        (Reg::Reg1, Bank::Bank0) => self.command(psx, value),
                        (Reg::Reg1, Bank::Bank1) => todo!(),
                        (Reg::Reg1, Bank::Bank2) => todo!(),
                        (Reg::Reg1, Bank::Bank3) => todo!(),

                        (Reg::Reg2, Bank::Bank0) => self.push_parameter(psx, value),
                        (Reg::Reg2, Bank::Bank1) => self.set_interrupt_mask(psx, value),
                        (Reg::Reg2, Bank::Bank2) => todo!(),
                        (Reg::Reg2, Bank::Bank3) => todo!(),

                        (Reg::Reg3, Bank::Bank0) => self.control_request(psx, value),
                        (Reg::Reg3, Bank::Bank1) => self.ack_interrupt_status(psx, value),
                        (Reg::Reg3, Bank::Bank2) => todo!(),
                        (Reg::Reg3, Bank::Bank3) => todo!(),
                    }
                }
            }
            Event::AckGeneric => {
                psx.cdrom.status.set_busy(false);
                psx.cdrom.result_queue.push_back(psx.cdrom.status.to_bits());
                self.interrupt_queue.push_back(InterruptKind::Acknowledge);
            }
            Event::AckInit => {
                psx.cdrom.status.set_busy(false);
                debug!(psx.loggers.cdrom, "INIT ack");
                psx.cdrom.result_queue.push_back(psx.cdrom.status.to_bits());
                self.interrupt_queue.push_back(InterruptKind::Acknowledge);
            }
            Event::CompleteInit => {
                debug!(psx.loggers.cdrom, "INIT complete");
                psx.cdrom.mode = Mode::from_bits(0x20);
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
