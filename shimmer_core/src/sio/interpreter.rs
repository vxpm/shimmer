use crate::{PSX, interrupts::Interrupt};
use tinylog::debug;

#[derive(Debug, Clone, Copy, Default)]
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
    fn transfer_joypad(&mut self, psx: &mut PSX) {
        debug!(psx.loggers.sio, "starting joypad transfer");

        self.transfer = Transfer::JoyPad(0);
        psx.sio.controllers[0]
            .rx_queue
            .extend([0x41, 0x5A, 0xFF, 0xFF]);
    }

    fn transfer_memcard(&mut self, psx: &mut PSX) {
        debug!(psx.loggers.sio, "starting memcard transfer");

        self.transfer = Transfer::Memcard(0);
        psx.sio.controllers[0].rx_queue.extend([0xFF]);
    }

    pub fn update(&mut self, psx: &mut PSX) {
        if psx.sio.controllers[0].control.acknowledge() {
            psx.sio.controllers[0].control.set_acknowledge(false);
            psx.sio.controllers[0].status.set_interrupt_request(false);
        }

        match &mut self.transfer {
            Transfer::None => {
                if let Some(address) = psx.sio.controllers[0].tx_queue.pop_front() {
                    match address {
                        0x01 => self.transfer_joypad(psx),
                        0x81 => self.transfer_memcard(psx),
                        _ => todo!("unknown address: {address:?}"),
                    }
                }
            }
            Transfer::JoyPad(index) => {
                if let Some(value) = psx.sio.controllers[0].tx_queue.pop_front() {
                    let i = *index;
                    *index += 1;

                    debug!(psx.loggers.sio, "reading from TX queue step {i}");
                    match i {
                        0 => assert_eq!(value, b'B'),
                        1 => assert_eq!(value, 0x00),
                        2 | 3 => {
                            self.transfer = Transfer::None;
                        }
                        _ => unreachable!(),
                    }

                    psx.sio.controllers[0].status.set_rx_input_level(false);
                }
            }
            Transfer::Memcard(_index) => {
                self.transfer = Transfer::None;
                psx.sio.controllers[0].status.set_rx_input_level(false);
            }
        }

        if psx.sio.controllers[0]
            .control
            .device_ready_to_send_interrupt_enable()
            && psx.sio.controllers[0].status.device_ready_to_send()
        {
            psx.interrupts.status.request(Interrupt::SIO);
            psx.sio.controllers[0].status.set_interrupt_request(true);
        }

        psx.sio.controllers[0].update_status();
    }
}
