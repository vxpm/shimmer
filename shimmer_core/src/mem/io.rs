use super::Address;
use strum::VariantArray;

#[derive(Debug, Clone, Copy, PartialEq, Eq, VariantArray)]
pub enum Reg {
    // Interrupt
    InterruptStatus,
    InterruptMask,

    // GPU
    Gp0,
    Gp1,

    // Etc
    Post,
}

impl Reg {
    fn address_and_width(self) -> (Address, usize) {
        let (addr, width) = match self {
            Reg::InterruptStatus => (0x1F80_1070, 4),
            Reg::InterruptMask => (0x1F80_1074, 4),

            Reg::Gp0 => (0x1F80_1810, 4),
            Reg::Gp1 => (0x1F80_1814, 4),

            Reg::Post => (0x1F80_2041, 1),
        };

        (Address(addr), width)
    }

    pub fn address(self) -> Address {
        self.address_and_width().0
    }

    pub fn width(self) -> usize {
        self.address_and_width().1
    }

    /// Returns the offset of the given address with respect to this register, but only if it's
    /// contained inside the register's range `addr..(addr + width)`.
    pub fn offset(self, addr: Address) -> Option<usize> {
        let (reg_addr, width) = self.address_and_width();
        addr.value()
            .checked_sub(reg_addr.value())
            .and_then(|offset| ((offset as usize) < width).then_some(offset as usize))
    }

    pub fn reg_and_offset(addr: Address) -> Option<(Reg, usize)> {
        for reg in Self::VARIANTS {
            if let Some(offset) = reg.offset(addr) {
                return Some((*reg, offset));
            }
        }

        None
    }
}
