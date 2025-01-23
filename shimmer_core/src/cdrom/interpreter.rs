mod command;
mod control;
mod interrupt;

use super::{Bank, Reg, RegWrite};
use crate::{PSX, cdrom::InterruptKind, interrupts::Interrupt};
use tinylog::debug;

fn trigger_cdrom_interrupt(psx: &mut PSX, kind: InterruptKind) {
    psx.cdrom.interrupt_status.set_kind(kind);
}

#[derive(Debug, Clone, Default)]
pub struct Interpreter {}

impl Interpreter {
    fn switch_bank(&mut self, psx: &mut PSX, bank: Bank) {
        psx.cdrom.status.set_bank(bank);
    }

    pub fn update(&mut self, psx: &mut PSX) {
        psx.cdrom.update_status();
        while let Some(RegWrite { reg, value }) = psx.cdrom.write_queue.pop_front() {
            if reg != Reg::Reg0 {
                debug!(
                    psx.loggers.cdrom,
                    "write to {:?}.{:?}: {:#02X}",
                    psx.cdrom.status.bank(),
                    reg,
                    value
                );
            }

            match (reg, psx.cdrom.status.bank()) {
                (Reg::Reg0, _) => {
                    let bank = Bank::from_repr(value as usize & 0b11).unwrap();
                    self.switch_bank(psx, bank);
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

        psx.cdrom.update_status();
        let masked =
            psx.cdrom.interrupt_status.kind() as u8 & psx.cdrom.interrupt_mask.mask().value();
        if masked != 0 {
            debug!(
                psx.loggers.cdrom,
                "requesting CDROM interrupt kind {:?}",
                psx.cdrom.interrupt_status.kind()
            );
            psx.interrupts.status.request(Interrupt::CDROM);
        }
    }
}
