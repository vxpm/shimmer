use crate::PSX;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Update,
}

#[derive(Debug, Clone, Default)]
pub struct Interpreter {}

impl Interpreter {
    pub fn update_status(&mut self, psx: &mut PSX) {
        psx.sio0.status.set_tx_ready(psx.sio0.tx.is_none());
        psx.sio0.status.set_rx_ready(psx.sio0.rx.is_some());
        psx.sio0.status.set_tx_finished(psx.sio0.rx.is_some()); // TODO: & not transmitting
    }

    pub fn update(&mut self, psx: &mut PSX, event: Event) {
        // do something
    }
}
