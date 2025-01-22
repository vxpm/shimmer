use super::{Bank, Command, RegWrite};
use crate::{
    PSX,
    cdrom::{InterruptKind, InterruptStatus, Mode},
    interrupts::Interrupt,
};
use bitos::{bitos, integer::u3};
use tinylog::{debug, trace};

#[bitos(8)]
struct HCHPCTL {
    #[bits(5)]
    sound_map_enable: bool,
    #[bits(6)]
    request_sector_buffer_write: bool,
    #[bits(7)]
    request_sector_buffer_read: bool,
}

#[bitos(8)]
struct HCLRCTL {
    #[bits(0..3)]
    acknowledge_interrupt_bits: u3,
    #[bits(3)]
    acknowledge_sound_buffer_empty: bool,
    #[bits(4)]
    acknowledge_sound_buffer_write_ready: bool,
    #[bits(5)]
    clear_sound_buffer: bool,
    #[bits(6)]
    clear_parameter_fifo: bool,
    #[bits(7)]
    reset_decoder: bool,
}

fn trigger_cdrom_interrupt(psx: &mut PSX, kind: InterruptKind) {
    let masked = psx.cdrom.interrupt_status.kind() as u8 & psx.cdrom.interrupt_mask.mask().value();
    if masked > 0 {
        psx.cdrom.interrupt_status.set_kind(kind);
        psx.interrupts.status.request(Interrupt::CDROM);
    }
}

#[derive(Debug, Clone, Default)]
pub struct Interpreter {}

impl Interpreter {
    fn command(&mut self, psx: &mut PSX, command: Command) {
        debug!(psx.loggers.cdrom, "received command {command:?}");

        match command {
            Command::Nop => {
                psx.cdrom.result_fifo.push(psx.cdrom.status.to_bits());
                trigger_cdrom_interrupt(psx, InterruptKind::Acknowledge);
            }
            Command::Init => {
                psx.cdrom.mode = Mode::from_bits(0x20);
                // TODO: activate drive motor? standby? abort all commands???
                trigger_cdrom_interrupt(psx, InterruptKind::Complete);
            }
            Command::Demute => {
                psx.cdrom.result_fifo.push(psx.cdrom.status.to_bits());
                trigger_cdrom_interrupt(psx, InterruptKind::Acknowledge);
            }
            _ => todo!("{:?}", command),
        }
    }

    fn write_reg0(&mut self, psx: &mut PSX, value: u8) {
        trace!(psx.loggers.cdrom, "switch to bank {}", value);
        psx.cdrom
            .status
            .set_bank(Bank::from_repr(value as usize & 0b11).unwrap());
    }

    fn write_reg1(&mut self, psx: &mut PSX, value: u8) {
        trace!(
            psx.loggers.cdrom,
            "{:?} reg 1: {:#02X}",
            psx.cdrom.status.bank(),
            value
        );

        match psx.cdrom.status.bank() {
            Bank::Bank0 => {
                self.command(psx, Command::new(value));
            }
            Bank::Bank1 => todo!(),
            Bank::Bank2 => todo!(),
            Bank::Bank3 => todo!(),
        }
    }

    fn write_reg2(&mut self, psx: &mut PSX, value: u8) {
        trace!(
            psx.loggers.cdrom,
            "{:?} reg 2: {:#02X}",
            psx.cdrom.status.bank(),
            value
        );

        match psx.cdrom.status.bank() {
            Bank::Bank0 => todo!(),
            Bank::Bank1 => {
                psx.cdrom.interrupt_status = InterruptStatus::from_bits(value | 0b1110_0000);
            }
            Bank::Bank2 => todo!(),
            Bank::Bank3 => todo!(),
        }
    }

    fn write_reg3(&mut self, psx: &mut PSX, value: u8) {
        trace!(
            psx.loggers.cdrom,
            "{:?} reg 3: {:#02X}",
            psx.cdrom.status.bank(),
            value
        );

        match psx.cdrom.status.bank() {
            Bank::Bank0 => {
                let _cmd = HCHPCTL::from_bits(value);
            }
            Bank::Bank1 => {
                let cmd = HCLRCTL::from_bits(value);

                // TODO: proper kind mask
                if cmd.acknowledge_interrupt_bits().value() > 0 {
                    psx.cdrom.interrupt_status.set_kind(InterruptKind::None);
                }
            }
            Bank::Bank2 => todo!(),
            Bank::Bank3 => todo!(),
        }
    }

    pub fn update(&mut self, psx: &mut PSX) {
        while let Some(write) = psx.cdrom.write_queue.pop_front() {
            match write {
                RegWrite::Reg0(value) => self.write_reg0(psx, value),
                RegWrite::Reg1(value) => self.write_reg1(psx, value),
                RegWrite::Reg2(value) => self.write_reg2(psx, value),
                RegWrite::Reg3(value) => self.write_reg3(psx, value),
            }
        }
    }
}
