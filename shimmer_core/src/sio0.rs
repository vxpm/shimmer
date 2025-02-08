mod interpreter;

use bitos::bitos;
pub use interpreter::{Event, Interpreter};

#[bitos(32)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Status {
    /// Whether the TX (PS1 -> Device) queue is not full.
    #[bits(0)]
    pub tx_ready: bool,
    /// Whether the RX (Device -> PS1) queue is not empty.
    #[bits(1)]
    pub rx_ready: bool,
    /// Whether the transmission has finished.
    #[bits(2)]
    pub tx_finished: bool,
    /// Whether the device is ready to receive more data. (DSR) (/ACK)
    #[bits(7)]
    pub device_ready_to_receive: bool,
    /// Whether an interrupt has been requested or not.
    #[bits(9)]
    pub interrupt_request: bool,
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

#[bitos(16)]
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
    /// The polarity of the clock. For SIO0, should always be disabled (high when idle).
    #[bits(8)]
    pub clock_polarity: bool,
}

#[bitos(16)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Control {
    /// Controls whether the PS1 can start a transfer to the device.
    #[bits(0)]
    pub tx_enable: bool,
    /// Whether this controller is selected for transmission. (DTR) (/CS)
    #[bits(1)]
    pub selected: bool,
    /// For SIO0, controls whether it will force a receive even if CS is high (not asserted).
    #[bits(2)]
    pub rx_enable: bool,
    /// Acknowledges the interrupt or a RX error.
    #[bits(4)]
    pub acknowledge: bool,
    /// Whether the PS1 is ready to receive data. (RTS)
    #[bits(5)]
    pub ready_to_receive: bool,
    /// Zeroes most registers (?).
    #[bits(6)]
    pub reset: bool,
    /// Whether to raise an interrupt on TX.
    #[bits(10)]
    pub tx_interrupt_enable: bool,
    /// Whether to raise an interrupt on RX.
    #[bits(11)]
    pub rx_interrupt_enable: bool,
    /// Whether to raise an interrupt when the device becomes ready to receive more data.
    #[bits(12)]
    pub device_ready_to_receive_interrupt_enable: bool,
    /// Selects which serial port to communicate with.
    #[bits(13)]
    pub port_select: bool,
}

#[bitos(16)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Input {
    #[bits(0)]
    pub select: bool,
    #[bits(1)]
    pub l3: bool,
    #[bits(2)]
    pub r3: bool,
    #[bits(3)]
    pub start: bool,
    #[bits(4)]
    pub joy_up: bool,
    #[bits(5)]
    pub joy_right: bool,
    #[bits(6)]
    pub joy_down: bool,
    #[bits(7)]
    pub joy_left: bool,
    #[bits(8)]
    pub l2: bool,
    #[bits(9)]
    pub r2: bool,
    #[bits(10)]
    pub l1: bool,
    #[bits(11)]
    pub r1: bool,
    #[bits(12)]
    pub triangle: bool,
    #[bits(13)]
    pub circle: bool,
    #[bits(14)]
    pub cross: bool,
    #[bits(15)]
    pub square: bool,
}

#[bitos(16)]
#[derive(Debug, Clone, Copy, Default)]
pub struct JoystickInput {
    #[bits(0..8)]
    pub joystick_x: u8,
    #[bits(8..16)]
    pub joystick_y: u8,
}

#[derive(Debug, Clone)]
pub struct Controller {
    pub status: Status,
    pub mode: Mode,
    pub control: Control,

    pub rx: Option<u8>,
    pub tx: Option<u8>,

    pub input: Input,
    pub left_joystick: JoystickInput,
    pub right_joystick: JoystickInput,
}

impl Default for Controller {
    fn default() -> Self {
        Self {
            status: Status::default().with_tx_ready(true).with_tx_ready(true),
            mode: Default::default(),
            control: Default::default(),

            rx: Default::default(),
            tx: Default::default(),

            input: Default::default(),
            left_joystick: Default::default(),
            right_joystick: Default::default(),
        }
    }
}

/// The state of the SIO0 controller.
impl Controller {
    pub fn read_rx(&mut self) -> u8 {
        self.rx.take().unwrap_or(0xFF)
    }
}
