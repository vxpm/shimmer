//! Items related to the system interrupt controller.

use bitos::bitos;
use strum::FromRepr;

/// A system interrupt source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
pub enum Interrupt {
    VBlank = 0x00,
    GPU = 0x01,
    CDROM = 0x02,
    DMA = 0x03,
    Timer0 = 0x04,
    Timer1 = 0x05,
    Timer2 = 0x06,
    ControllerAndMemCard = 0x07,
    SIO = 0x08,
    SPU = 0x09,
    Controller = 0xA,
}

/// Register which contains which system interrupts are currently pending.
#[bitos(32)]
#[derive(Clone, Copy, Default)]
pub struct Status {
    #[bits(0..10)]
    status: [bool; 10],
}

impl std::fmt::Debug for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set()
            .entries(
                self.status()
                    .iter()
                    .enumerate()
                    .filter_map(|(i, requested)| {
                        requested.then_some(Interrupt::from_repr(i).unwrap())
                    }),
            )
            .finish()
    }
}

impl Status {
    /// Requests the given system interrupt.
    #[inline(always)]
    pub fn request(&mut self, interrupt: Interrupt) {
        self.set_status_at(interrupt as usize, true);
    }

    /// Returns a [`Status`] masked with the given [`Mask`].
    #[inline(always)]
    pub fn mask(&mut self, mask: &Mask) -> Self {
        let status_bits: u32 = self.0;
        let mask_bits: u32 = mask.0;
        let masked = status_bits & mask_bits;

        Self::from_bits(masked)
    }

    /// Returns a requested [`Interrupt`], if any.
    #[inline(always)]
    pub fn requested(&self) -> Option<Interrupt> {
        // TODO: improve this
        let trailing = self.0.trailing_zeros();
        (trailing < 10).then(|| match trailing {
            0x00 => Interrupt::VBlank,
            0x01 => Interrupt::GPU,
            0x02 => Interrupt::CDROM,
            0x03 => Interrupt::DMA,
            0x04 => Interrupt::Timer0,
            0x05 => Interrupt::Timer1,
            0x06 => Interrupt::Timer2,
            0x07 => Interrupt::ControllerAndMemCard,
            0x08 => Interrupt::SIO,
            0x09 => Interrupt::SPU,
            0x0A => Interrupt::Controller,
            _ => unreachable!(),
        })
    }
}

/// Register which contains which system interrupts are allowed to be raised.
#[bitos(32)]
#[derive(Clone, Copy, Default)]
pub struct Mask {
    #[bits(0..10)]
    enabled: [bool; 10],
}

impl std::fmt::Debug for Mask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set()
            .entries(
                self.enabled()
                    .iter()
                    .enumerate()
                    .filter_map(|(i, enabled)| enabled.then_some(Interrupt::from_repr(i).unwrap())),
            )
            .finish()
    }
}

/// The state of the interrupt controller.
#[derive(Debug, Clone, Default)]
pub struct Controller {
    pub status: Status,
    pub mask: Mask,
}
