//! Items related to the timers of the PSX.

use bitos::{bitos, integer::u2};

#[bitos(2)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncModeC0C1 {
    PauseAtBlank,
    ResetAtBlank,
    ResetAtBlankAndPauseOutside,
    PauseUntilBlankThenNoSync,
}

#[bitos(2)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncModeC2 {
    StopCounter,
    NoSync,
}

#[bitos(1)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqRepeatMode {
    Oneshot,
    Repeat,
}

#[bitos(1)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqToggleMode {
    Pulse,
    Toggle,
}

#[bitos(32)]
#[derive(Debug, Clone, Copy, Default)]
pub struct TimerMode {
    /// Whether synchronization is enabled or not.
    #[bits(0)]
    pub sync: bool,

    /// The synchronization mode, if it is enabled.
    #[bits(1..3)]
    pub sync_mode: u2,

    /// Whether the timer value should reset once the target value has been reached.
    #[bits(3)]
    pub reset_at_target: bool,
    /// Whether a IRQ should be fired once the target value has been reached.
    #[bits(4)]
    pub irq_when_at_target: bool,
    /// Whether a IRQ should be fired once the timer's value has reached 0xFFFF.
    #[bits(5)]
    pub irq_at_max: bool,
    /// How the IRQ should repeat.
    #[bits(6)]
    pub irq_repeat_mode: IrqRepeatMode,
    /// How the IRQ should be toggled.
    #[bits(7)]
    pub irq_toggle_mode: IrqToggleMode,

    /// The source of the timer's clock.
    #[bits(8..10)]
    pub clock_source: u2,

    /// Whether an interrupt has been requested.
    #[bits(10)]
    pub no_irq: bool,
    /// Whether the target has been reached since the last time this register was read. Resets on
    /// read.
    #[bits(11)]
    pub reached_target: bool,
    /// Whether the timer's value has reached 0xFFFF since the last time this register was read.
    /// Resets on read.
    #[bits(12)]
    pub reached_max: bool,
}

#[derive(Default)]
pub struct Timer1 {
    pub value: u16,
    pub target: u16,
    pub mode: TimerMode,
}

impl Timer1 {
    pub fn should_tick(&self) -> bool {
        !self.mode.sync() || matches!(self.mode.sync_mode().value(), 1 | 2)
    }

    pub fn can_raise_irq(&self) -> bool {
        match self.mode.irq_repeat_mode() {
            IrqRepeatMode::Oneshot => self.mode.no_irq(),
            IrqRepeatMode::Repeat => true,
        }
    }

    pub fn update_no_irq(&mut self) {
        match self.mode.irq_toggle_mode() {
            IrqToggleMode::Pulse => {
                self.mode.set_no_irq(false);
            }
            IrqToggleMode::Toggle => {
                self.mode.set_no_irq(!self.mode.no_irq());
            }
        }
    }

    pub fn cycles_per_tick(&self) -> u64 {
        if self.mode.clock_source().value() < 2 {
            2
        } else {
            16
        }
    }
}

#[derive(Default)]
pub struct Timer2 {
    pub value: u16,
    pub target: u16,
    pub mode: TimerMode,
}

impl Timer2 {
    pub fn should_tick(&self) -> bool {
        !self.mode.sync() || matches!(self.mode.sync_mode().value(), 1 | 2)
    }

    pub fn can_raise_irq(&self) -> bool {
        match self.mode.irq_repeat_mode() {
            IrqRepeatMode::Oneshot => self.mode.no_irq(),
            IrqRepeatMode::Repeat => true,
        }
    }

    pub fn update_no_irq(&mut self) {
        match self.mode.irq_toggle_mode() {
            IrqToggleMode::Pulse => {
                self.mode.set_no_irq(false);
            }
            IrqToggleMode::Toggle => {
                self.mode.set_no_irq(!self.mode.no_irq());
            }
        }
    }

    pub fn cycles_per_tick(&self) -> u64 {
        if self.mode.clock_source().value() < 2 {
            2
        } else {
            16
        }
    }
}

#[derive(Default)]
pub struct Timers {
    pub timer1: Timer1,
    pub timer2: Timer2,
}
