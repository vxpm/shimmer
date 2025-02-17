use crate::{PSX, scheduler};
use shimmer_core::interrupts::Interrupt;
use tinylog::{debug, trace};

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
    JoypadTransfer(JoypadStage),
}

#[derive(Debug, Clone, Default)]
pub struct Sio0 {
    state: State,
    in_progress: bool,
}

const TRANSFER_DELAY: u64 = 1500;
const START_ACK_DELAY: u64 = 100;
const END_ACK_DELAY: u64 = 50;

impl Sio0 {
    fn update_status(&mut self, psx: &mut PSX) {
        psx.sio0.status.set_tx_ready(psx.sio0.tx.is_none());
        psx.sio0.status.set_rx_ready(psx.sio0.rx.is_some());
        psx.sio0
            .status
            .set_tx_finished(psx.sio0.tx.is_none() && !self.in_progress);
    }

    fn can_transfer(&mut self, psx: &mut PSX) -> bool {
        psx.sio0.control.selected()
            && psx.sio0.control.tx_enable()
            && psx.sio0.tx.is_some()
            && !self.in_progress
    }

    pub fn update(&mut self, psx: &mut PSX, event: Event) {
        self.update_status(psx);

        if psx.sio0.control.acknowledge() {
            psx.sio0.control.set_acknowledge(false);
            psx.sio0.status.set_interrupt_request(false);
        }

        // do something
        match (self.state, event) {
            (_, Event::Update) => {
                // check if a transfer should start
                if self.can_transfer(psx) {
                    self.in_progress = true;
                    psx.scheduler
                        .schedule(scheduler::Event::Sio(Event::Transfer), TRANSFER_DELAY);
                }
            }
            (_, Event::StartAck) => {
                trace!(psx.loggers.sio, "start ack");
                psx.sio0.status.set_device_ready_to_receive(true);
                psx.scheduler
                    .schedule(scheduler::Event::Sio(Event::EndAck), END_ACK_DELAY);

                if psx.sio0.control.device_ready_to_receive_interrupt_enable() {
                    psx.sio0.status.set_interrupt_request(true);
                    psx.interrupts
                        .status
                        .request(Interrupt::ControllerAndMemCard);
                }
            }
            (_, Event::EndAck) => {
                trace!(psx.loggers.sio, "end ack");
                psx.sio0.status.set_device_ready_to_receive(false);
            }
            (State::Idle, Event::Transfer) => {
                self.in_progress = false;
                psx.sio0.rx = Some(0xFF);

                let address = psx.sio0.tx.take().unwrap();
                match address {
                    0x01 if !psx.sio0.control.port_select() => {
                        // joypad
                        psx.scheduler
                            .schedule(scheduler::Event::Sio(Event::StartAck), START_ACK_DELAY);
                        self.state = State::JoypadTransfer(JoypadStage::IdLow);
                    }
                    _ => {}
                }
            }
            (State::JoypadTransfer(stage), Event::Transfer) => {
                self.in_progress = false;

                let data = psx.sio0.tx.take().unwrap();
                match stage {
                    JoypadStage::IdLow => {
                        debug!(psx.loggers.sio, "sending ID low");
                        assert!(matches!(data, b'B' | b'C'), "data is unexpected: {data}");
                        psx.sio0.rx = Some(0x41);
                        psx.scheduler
                            .schedule(scheduler::Event::Sio(Event::StartAck), START_ACK_DELAY);
                        self.state = State::JoypadTransfer(JoypadStage::IdHigh);
                    }
                    JoypadStage::IdHigh => {
                        debug!(psx.loggers.sio, "sending ID high");
                        assert_eq!(data, 0x00); // TAP
                        psx.sio0.rx = Some(0x5A);
                        psx.scheduler
                            .schedule(scheduler::Event::Sio(Event::StartAck), START_ACK_DELAY);
                        self.state = State::JoypadTransfer(JoypadStage::Rumble0);
                    }
                    JoypadStage::Rumble0 => {
                        debug!(psx.loggers.sio, "sending switches low");
                        psx.sio0.rx = Some(!psx.sio0.input.to_bits().to_le_bytes()[0]);
                        psx.scheduler
                            .schedule(scheduler::Event::Sio(Event::StartAck), START_ACK_DELAY);
                        self.state = State::JoypadTransfer(JoypadStage::Rumble1);
                    }
                    JoypadStage::Rumble1 => {
                        debug!(psx.loggers.sio, "sending switches high");
                        psx.sio0.rx = Some(!psx.sio0.input.to_bits().to_le_bytes()[1]);
                        self.state = State::Idle;
                    }
                }
            }
        }

        self.update_status(psx);
    }
}
