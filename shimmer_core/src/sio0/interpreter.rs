use super::Snapshot;
use crate::{PSX, cpu, scheduler};
use tinylog::trace;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Update,
    Transfer,
    StartAck,
    EndAck,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoypadStage {
    IdLow,
    IdHigh,
    Rumble0,
    Rumble1,
}

#[derive(Debug, Clone, Copy, Default)]
enum State {
    #[default]
    Idle,
    StartTransfer(u8),
    JoypadTransfer(JoypadStage),
}

#[derive(Debug, Clone, Default)]
pub struct Interpreter {
    state: State,
}

const DELAY: u64 = 3 * cpu::CYCLES_1_US as u64;

impl Interpreter {
    fn snap(&mut self, psx: &mut PSX) {
        psx.sio0.snaps.push(Snapshot {
            cycle: psx.scheduler.elapsed(),
            status: psx.sio0.status,
            mode: psx.sio0.mode,
            control: psx.sio0.control,
            tx: psx.sio0.tx,
            rx: psx.sio0.rx,
        });
    }

    fn update_status(&mut self, psx: &mut PSX) {
        psx.sio0.status.set_tx_ready(psx.sio0.tx.is_none());
        psx.sio0.status.set_rx_ready(psx.sio0.rx.is_some());
        psx.sio0
            .status
            .set_tx_finished(psx.sio0.tx.is_none() && matches!(self.state, State::Idle));
    }

    fn can_transfer(&mut self, psx: &mut PSX) -> bool {
        psx.sio0.control.selected() && psx.sio0.control.tx_enable() && psx.sio0.tx.is_some()
    }

    pub fn update(&mut self, psx: &mut PSX, event: Event) {
        self.snap(psx);
        self.update_status(psx);

        // do something
        match (self.state, event) {
            (State::Idle, Event::Update) => {
                // check if a transfer should start
                if self.can_transfer(psx) {
                    let data = psx.sio0.tx.take().unwrap();
                    self.state = State::StartTransfer(data);
                    psx.scheduler
                        .schedule(scheduler::Event::Sio(Event::Transfer), DELAY);
                }
            }
            (_, Event::StartAck) => {
                trace!(psx.loggers.sio, "start ack");
                psx.sio0.status.set_device_ready_to_receive(true);
                psx.scheduler
                    .schedule(scheduler::Event::Sio(Event::EndAck), DELAY);

                if psx.sio0.control.device_ready_to_receive_interrupt_enable() {
                    psx.interrupts
                        .status
                        .request(crate::interrupts::Interrupt::SIO);
                    psx.sio0.status.set_interrupt_request(true);
                }
            }
            (_, Event::EndAck) => {
                trace!(psx.loggers.sio, "end ack");
                psx.sio0.status.set_device_ready_to_receive(false);
                psx.scheduler
                    .schedule(scheduler::Event::Sio(Event::Transfer), DELAY);
            }
            (State::StartTransfer(value), Event::Transfer) => {
                match value {
                    0x01 => {
                        // joypad
                        self.state = State::JoypadTransfer(JoypadStage::IdLow);
                        psx.sio0.rx = Some(0xFF);
                        psx.scheduler
                            .schedule(scheduler::Event::Sio(Event::StartAck), DELAY);
                    }
                    0x81 => {
                        // memcard
                        todo!("memcard transfer")
                    }
                    _ => todo!("unknown device"),
                }
            }
            (State::JoypadTransfer(stage), Event::Update | Event::Transfer) => {
                if self.can_transfer(psx) {
                    let data = psx.sio0.tx.take().unwrap();
                    match stage {
                        JoypadStage::IdLow => assert_eq!(data, 0x43),
                        JoypadStage::IdHigh => todo!(),
                        JoypadStage::Rumble0 => todo!(),
                        JoypadStage::Rumble1 => todo!(),
                    }
                }
            }
            _ => (),
        }

        self.update_status(psx);
        self.snap(psx);
    }
}
