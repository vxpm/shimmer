use crate::{PSX, cpu, interrupts::Interrupt, scheduler};
use tinylog::debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Update,
    FinishTransmission,
    StartAck,
    FinishAck,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Transfer {
    #[default]
    None,
    JoyPad(u8),
    Memcard(u8),
}

#[derive(Debug, Clone, Default)]
pub struct Interpreter {
    transfer: Transfer,
}

impl Interpreter {
    pub fn update_status(&mut self, psx: &mut PSX) {
        psx.sio0.status.set_tx_ready(psx.sio0.tx.is_none());
        psx.sio0.status.set_rx_ready(psx.sio0.rx.is_some());
        psx.sio0.status.set_tx_finished(psx.sio0.rx.is_some()); // TODO: & not transmitting
    }

    fn start_ack(&self, psx: &mut PSX) {
        psx.sio0.status.set_device_ready_to_receive(true);
    }

    fn finish_ack(&self, psx: &mut PSX) {
        psx.sio0.status.set_device_ready_to_receive(false);
    }

    fn transfer_joypad(&mut self, psx: &mut PSX) {
        debug!(psx.loggers.sio, "starting joypad transfer");
        self.transfer = Transfer::JoyPad(0);
        psx.sio0.control.set_rx_enable(true); // TODO: idk?

        psx.scheduler.schedule(
            scheduler::Event::Sio(Event::FinishTransmission),
            cpu::FREQUENCY as u64 / (250 * 1000), // HACK: constant
        );
    }

    fn transfer_memcard(&mut self, psx: &mut PSX) {
        debug!(psx.loggers.sio, "starting memcard transfer");
        psx.sio0.status.set_device_ready_to_receive(false);
    }

    pub fn update(&mut self, psx: &mut PSX, event: Event) {
        self.update_status(psx);

        match &mut self.transfer {
            Transfer::None => {
                if let Some(address) = psx.sio0.tx
                    && psx.sio0.control.tx_enable()
                    && psx.sio0.control.selected()
                {
                    psx.sio0.tx = None;
                    match address {
                        0x01 => self.transfer_joypad(psx),
                        0x81 => self.transfer_memcard(psx),
                        _ => todo!("unknown address: {address:?}"),
                    }
                }
            }
            Transfer::JoyPad(index) => match event {
                Event::Update => (),
                Event::FinishTransmission => {
                    let received = psx.sio0.tx.take().unwrap_or(0xFF);
                    debug!(
                        psx.loggers.sio,
                        "received value 0x{:02X} (index {})", received, *index
                    );

                    psx.scheduler.schedule(
                        scheduler::Event::Sio(Event::StartAck),
                        3 * cpu::CYCLES_1_US as u64,
                    );

                    if psx.sio0.control.device_ready_to_receive_interrupt_enable() {
                        debug!(psx.loggers.sio, "requested SIO interrupt");
                        psx.sio0.status.set_interrupt_request(true);
                        psx.interrupts.status.request(Interrupt::SIO);
                    }
                }
                Event::StartAck => {
                    debug!(psx.loggers.sio, "starting ack pulse {}", *index,);
                    self.start_ack(psx);

                    psx.scheduler.schedule(
                        scheduler::Event::Sio(Event::FinishAck),
                        3 * cpu::CYCLES_1_US as u64,
                    );
                }
                Event::FinishAck => {
                    debug!(psx.loggers.sio, "finishing joypad ack pulse {}", *index);

                    match index {
                        0 => {
                            psx.sio0.rx = Some(0xff);
                            debug!(psx.loggers.sio, "set rx to 0x{:02X}", psx.sio0.rx.unwrap());
                            *index += 1;
                        }
                        1 => {
                            psx.sio0.rx = Some(0x41);
                            debug!(psx.loggers.sio, "set rx to 0x{:02X}", psx.sio0.rx.unwrap());
                            *index += 1;
                        }
                        2 => {
                            psx.sio0.rx = Some(0x5A);
                            debug!(psx.loggers.sio, "set rx to 0x{:02X}", psx.sio0.rx.unwrap());
                            *index += 1;
                        }
                        3 => {
                            psx.sio0.rx = Some(0xFF);
                            debug!(psx.loggers.sio, "set rx to 0x{:02X}", psx.sio0.rx.unwrap());
                            *index += 1;
                        }
                        4 => {
                            psx.sio0.rx = Some(0xFF);
                            debug!(psx.loggers.sio, "set rx to 0x{:02X}", psx.sio0.rx.unwrap());
                            debug!(psx.loggers.sio, "finished joypad transfer");
                            self.transfer = Transfer::None;
                        }
                        _ => unreachable!(),
                    }

                    self.finish_ack(psx);
                }
            },
            Transfer::Memcard(_index) => unreachable!(),
        }

        if psx.sio0.control.acknowledge() {
            psx.sio0.control.set_acknowledge(false);
            psx.sio0.status.set_interrupt_request(false);
        }

        self.update_status(psx);
    }
}
