use bitos::{bitos, integer::u21};

#[bitos(32)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Status {
    #[bits(0)]
    pub tx_not_full: bool,
    #[bits(1)]
    pub rx_not_empty: bool,
    #[bits(2)]
    pub tx_idle: bool,
    #[bits(3)]
    pub rx_parity_error: bool,
    #[bits(4)]
    pub rx_overrun: bool, // SIO1 only
    #[bits(5)]
    pub rx_bad_stop_bit: bool, // SIO1 only
    #[bits(6)]
    pub rx_input_level: bool, // SIO1 only
    #[bits(7)]
    pub dsr_input_level: bool,
    #[bits(8)]
    pub cts_input_level: bool, // SIO1 only
    #[bits(9)]
    pub interrupt_request: bool,
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
    #[bits(0..2)]
    pub baudrate_factor: ReloadFactor,
    #[bits(2..4)]
    pub character_length: CharacterLength,
    #[bits(4)]
    pub parity_enable: bool,
    #[bits(5)]
    pub parity_odd: bool,
    #[bits(6..8)]
    pub stop_bit_length: StopLength, // SIO1 only
    #[bits(8)]
    pub clock_polarity: bool, // SIO0 only
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
    #[bits(0)]
    pub tx_enable: bool,
    #[bits(1)]
    pub dtr_output_level: bool,
    #[bits(2)]
    pub rx_enable: bool,
    #[bits(3)]
    pub tx_output_level: bool, // SIO1 only
    #[bits(4)]
    pub acknowledge: bool,
    #[bits(5)]
    pub rts_output_level: bool, // SIO1 only
    #[bits(6)]
    pub reset: bool,
    #[bits(8..10)]
    pub rx_interrupt_mode: InterruptMode,
    #[bits(10)]
    pub tx_interrupt_enable: bool,
    #[bits(11)]
    pub rx_interrupt_enable: bool,
    #[bits(12)]
    pub dsr_interrupt_enable: bool,
    #[bits(13)]
    pub port_select: bool, // SIO0 only
}

#[derive(Debug, Clone, Default)]
pub struct Controller {
    pub status: Status,
    pub mode: Mode,
    pub control: Control,
}

/// The state of the SIO interface.
#[derive(Debug, Clone, Default)]
pub struct Interface {
    pub controllers: [Controller; 2],
}
