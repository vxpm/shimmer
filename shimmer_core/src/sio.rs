mod interpreter;

use bitos::{bitos, integer::u21};
use std::collections::VecDeque;

pub use interpreter::Interpreter;

#[bitos(32)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Status {
    /// Whether the TX (PS1 -> Device) queue is not full.
    #[bits(0)]
    pub tx_not_full: bool,
    /// Whether the RX (Device -> PS1) queue is not empty.
    #[bits(1)]
    pub rx_not_empty: bool,
    /// TODO
    #[bits(2)]
    pub tx_idle: bool,
    // #[bits(3)]
    // pub rx_parity_error: bool,
    // #[bits(4)]
    // pub rx_overrun: bool, // SIO1 only
    // #[bits(5)]
    // pub rx_bad_stop_bit: bool, // SIO1 only
    // #[bits(6)]
    // pub rx_input_level: bool, // SIO1 only
    /// Whether the device is ready to send data. (DSR)
    #[bits(7)]
    pub device_ready_to_send: bool,
    /// Whether the device is ready to receive data. (CTS)
    #[bits(8)]
    pub device_ready_to_receive: bool, // SIO1 only
    /// Whether an interrupt is currently requested.
    #[bits(9)]
    pub interrupt_request: bool,
    /// A baudrate timer.
    #[bits(11..32)]
    pub timer: u21,
}

#[bitos(2)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadFactor {
    Times1OrStop,
    Times1,
    Times16,
    Times64,
}

#[bitos(2)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterLength {
    B5,
    B6,
    B7,
    B8,
}

#[bitos(2)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopLength {
    Reserved,
    B1,
    B1_5,
    B2,
}

#[bitos(32)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Mode {
    /// A factor for which to multiply the baudrate by.
    #[bits(0..2)]
    pub baudrate_factor: ReloadFactor,
    /// The character length of the serial transmission. For SIO0, should always be 8 bits.
    #[bits(2..4)]
    pub character_length: CharacterLength,
    /// Whether the transmission contains parity bits or not. For SIO0, should always be disabled.
    #[bits(4)]
    pub parity_enable: bool,
    /// Whether the parity indicates the amount of even or odd bits.
    #[bits(5)]
    pub parity_odd: bool,
    /// The length of the stop bit. For SIO0, always zero (it has no stop bit).
    #[bits(6..8)]
    pub stop_bit_length: StopLength, // SIO1 only
    /// The polarity of the clock. For SIO0, should always be disabled (high when idle).
    #[bits(8)]
    pub clock_polarity: bool,
}

#[bitos(2)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptMode {
    QueueLength1,
    QueueLength2,
    QueueLength4,
    QueueLength8,
}

#[bitos(32)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Control {
    /// Controls whether the PS1 can start a transfer to the device.
    #[bits(0)]
    pub tx_enable: bool,
    /// Whether the PS1 is ready to send data. (DTR)
    #[bits(1)]
    pub ready_to_send: bool,
    /// For SIO0, controls whether it will force a receive even if CS is high (not asserted).
    #[bits(2)]
    pub rx_enable: bool,
    // #[bits(3)]
    // pub tx_output_level: bool, // SIO1 only
    /// Acknowledges the interrupt or a RX error.
    #[bits(4)]
    pub acknowledge: bool,
    /// Whether the PS1 is ready to receive data. (RTS)
    #[bits(5)]
    pub ready_to_receive: bool, // SIO1 only
    /// Zeroes most registers (?).
    #[bits(6)]
    pub reset: bool,
    /// Controls when to raise an interrupt for RX.
    #[bits(8..10)]
    pub rx_interrupt_mode: InterruptMode,
    /// Whether to raise an interrupt on TX.
    #[bits(10)]
    pub tx_interrupt_enable: bool,
    /// Whether to raise an interrupt on RX.
    #[bits(11)]
    pub rx_interrupt_enable: bool,
    /// Whether to raise an interrupt when the device becomes ready to send.
    #[bits(12)]
    pub device_ready_to_send_interrupt_enable: bool,
    /// For SIO0, selects which serial port to communicate with.
    #[bits(13)]
    pub port_select: bool, // SIO0 only
}

#[derive(Debug, Clone)]
pub struct Controller {
    pub status: Status,
    pub mode: Mode,
    pub control: Control,

    pub tx_queue: VecDeque<u8>,
    pub rx_queue: VecDeque<u8>,
}

impl Default for Controller {
    fn default() -> Self {
        Self {
            status: Status::default().with_tx_not_full(true).with_tx_idle(true),
            mode: Default::default(),
            control: Default::default(),

            tx_queue: Default::default(),
            rx_queue: Default::default(),
        }
    }
}

impl Controller {
    pub fn update_status(&mut self) {
        self.status.set_rx_not_empty(!self.rx_queue.is_empty());
    }
}

/// The state of the SIO interface.
#[derive(Debug, Clone, Default)]
pub struct Interface {
    pub controllers: [Controller; 2],
}
