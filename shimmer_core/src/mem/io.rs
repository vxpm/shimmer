//! Items related to memory mapped IO.

use super::{Address, PhysicalAddress};
use crate::{cdrom, dma};
use strum::VariantArray;

/// A memory mapped register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, VariantArray)]
pub enum Reg {
    // Memory Control 1
    Expansion1Base,
    Expansion2Base,
    Expansion1Delay,
    Expansion3Delay,
    BiosDelay,
    SpuDelay,
    CdromDelay,
    Expansion2Delay,
    CommonDelay,

    // Peripheral IO
    JoyData,
    JoyStat,
    JoyMode,
    JoyControl,
    JoyBaud,

    // Memory Control 2
    RamSize,

    // Interrupt
    InterruptStatus,
    InterruptMask,

    // DMA
    Dma0Base,
    Dma0BlockControl,
    Dma0Control,

    Dma1Base,
    Dma1BlockControl,
    Dma1Control,

    Dma2Base,
    Dma2BlockControl,
    Dma2Control,

    Dma3Base,
    Dma3BlockControl,
    Dma3Control,

    Dma4Base,
    Dma4BlockControl,
    Dma4Control,

    Dma5Base,
    Dma5BlockControl,
    Dma5Control,

    Dma6Base,
    Dma6BlockControl,
    Dma6Control,

    DmaControl,
    DmaInterrupt,

    // Timers
    Timer0Value,
    Timer0Mode,
    Timer0Target,

    Timer1Value,
    Timer1Mode,
    Timer1Target,

    Timer2Value,
    Timer2Mode,
    Timer2Target,

    // CDROM
    Cdrom0,
    Cdrom1,
    Cdrom2,
    Cdrom3,

    // GPU
    Gp0,
    Gp1,

    // MDEC
    MdecCommand,
    MdecStatus,

    // SPU Voice {{{
    Voice0Volume,
    Voice0SampleRate,
    Voice0Start,
    Voice0ADSR,
    Voice0ADSRVolume,
    Voice0Repeat,

    Voice1Volume,
    Voice1SampleRate,
    Voice1Start,
    Voice1ADSR,
    Voice1ADSRVolume,
    Voice1Repeat,

    Voice2Volume,
    Voice2SampleRate,
    Voice2Start,
    Voice2ADSR,
    Voice2ADSRVolume,
    Voice2Repeat,

    Voice3Volume,
    Voice3SampleRate,
    Voice3Start,
    Voice3ADSR,
    Voice3ADSRVolume,
    Voice3Repeat,

    Voice4Volume,
    Voice4SampleRate,
    Voice4Start,
    Voice4ADSR,
    Voice4ADSRVolume,
    Voice4Repeat,

    Voice5Volume,
    Voice5SampleRate,
    Voice5Start,
    Voice5ADSR,
    Voice5ADSRVolume,
    Voice5Repeat,

    Voice6Volume,
    Voice6SampleRate,
    Voice6Start,
    Voice6ADSR,
    Voice6ADSRVolume,
    Voice6Repeat,

    Voice7Volume,
    Voice7SampleRate,
    Voice7Start,
    Voice7ADSR,
    Voice7ADSRVolume,
    Voice7Repeat,

    Voice8Volume,
    Voice8SampleRate,
    Voice8Start,
    Voice8ADSR,
    Voice8ADSRVolume,
    Voice8Repeat,

    Voice9Volume,
    Voice9SampleRate,
    Voice9Start,
    Voice9ADSR,
    Voice9ADSRVolume,
    Voice9Repeat,

    Voice10Volume,
    Voice10SampleRate,
    Voice10Start,
    Voice10ADSR,
    Voice10ADSRVolume,
    Voice10Repeat,

    Voice11Volume,
    Voice11SampleRate,
    Voice11Start,
    Voice11ADSR,
    Voice11ADSRVolume,
    Voice11Repeat,

    Voice12Volume,
    Voice12SampleRate,
    Voice12Start,
    Voice12ADSR,
    Voice12ADSRVolume,
    Voice12Repeat,

    Voice13Volume,
    Voice13SampleRate,
    Voice13Start,
    Voice13ADSR,
    Voice13ADSRVolume,
    Voice13Repeat,

    Voice14Volume,
    Voice14SampleRate,
    Voice14Start,
    Voice14ADSR,
    Voice14ADSRVolume,
    Voice14Repeat,

    Voice15Volume,
    Voice15SampleRate,
    Voice15Start,
    Voice15ADSR,
    Voice15ADSRVolume,
    Voice15Repeat,

    Voice16Volume,
    Voice16SampleRate,
    Voice16Start,
    Voice16ADSR,
    Voice16ADSRVolume,
    Voice16Repeat,

    Voice17Volume,
    Voice17SampleRate,
    Voice17Start,
    Voice17ADSR,
    Voice17ADSRVolume,
    Voice17Repeat,

    Voice18Volume,
    Voice18SampleRate,
    Voice18Start,
    Voice18ADSR,
    Voice18ADSRVolume,
    Voice18Repeat,

    Voice19Volume,
    Voice19SampleRate,
    Voice19Start,
    Voice19ADSR,
    Voice19ADSRVolume,
    Voice19Repeat,

    Voice20Volume,
    Voice20SampleRate,
    Voice20Start,
    Voice20ADSR,
    Voice20ADSRVolume,
    Voice20Repeat,

    Voice21Volume,
    Voice21SampleRate,
    Voice21Start,
    Voice21ADSR,
    Voice21ADSRVolume,
    Voice21Repeat,

    Voice22Volume,
    Voice22SampleRate,
    Voice22Start,
    Voice22ADSR,
    Voice22ADSRVolume,
    Voice22Repeat,

    Voice23Volume,
    Voice23SampleRate,
    Voice23Start,
    Voice23ADSR,
    Voice23ADSRVolume,
    Voice23Repeat,
    // }}}

    // SPU Control
    MainVolume,
    ReverbVolume,

    VoiceKeyOn,
    VoiceKeyOff,
    VoiceChannelFmMode,
    VoiceChannelNoiseMode,
    VoiceChannelReverbMode,
    VoiceChannelEnabled,

    SramReverbAddress,
    SramInterruptAddress,
    SramAddress,
    SramFifo,
    SpuControl,
    SramControl,
    SpuStatus,

    CdVolume,
    ExternVolume,

    // Expansion Region 2
    Post,
}

impl Reg {
    /// Returns the address and the width of this register.
    pub const fn address_and_width(self) -> (PhysicalAddress, usize) {
        let (addr, width) = match self {
            // Memory Control 1
            Reg::Expansion1Base => (0x1F80_1000, 4),
            Reg::Expansion2Base => (0x1F80_1004, 4),
            Reg::Expansion1Delay => (0x1F80_1008, 4),
            Reg::Expansion3Delay => (0x1F80_100C, 4),
            Reg::BiosDelay => (0x1F80_1010, 4),
            Reg::SpuDelay => (0x1F80_1014, 4),
            Reg::CdromDelay => (0x1F80_1018, 4),
            Reg::Expansion2Delay => (0x1F80_101C, 4),
            Reg::CommonDelay => (0x1F80_1020, 4),

            // Peripheral IO
            Reg::JoyData => (0x1F80_1040, 4),
            Reg::JoyStat => (0x1F80_1044, 4),
            Reg::JoyMode => (0x1F80_1048, 2),
            Reg::JoyControl => (0x1F80_104A, 2),
            Reg::JoyBaud => (0x1F80_104E, 2),

            // Memory Control 2
            Reg::RamSize => (0x1F80_1060, 4),

            // Interrupt
            Reg::InterruptStatus => (0x1F80_1070, 4),
            Reg::InterruptMask => (0x1F80_1074, 4),

            // DMA
            Reg::Dma0Base => (0x1F80_1080, 4),
            Reg::Dma0BlockControl => (0x1F80_1084, 4),
            Reg::Dma0Control => (0x1F80_1088, 4),

            Reg::Dma1Base => (0x1F80_1090, 4),
            Reg::Dma1BlockControl => (0x1F80_1094, 4),
            Reg::Dma1Control => (0x1F80_1098, 4),

            Reg::Dma2Base => (0x1F80_10A0, 4),
            Reg::Dma2BlockControl => (0x1F80_10A4, 4),
            Reg::Dma2Control => (0x1F80_10A8, 4),

            Reg::Dma3Base => (0x1F80_10B0, 4),
            Reg::Dma3BlockControl => (0x1F80_10B4, 4),
            Reg::Dma3Control => (0x1F80_10B8, 4),

            Reg::Dma4Base => (0x1F80_10C0, 4),
            Reg::Dma4BlockControl => (0x1F80_10C4, 4),
            Reg::Dma4Control => (0x1F80_10C8, 4),

            Reg::Dma5Base => (0x1F80_10D0, 4),
            Reg::Dma5BlockControl => (0x1F80_10D4, 4),
            Reg::Dma5Control => (0x1F80_10D8, 4),

            Reg::Dma6Base => (0x1F80_10E0, 4),
            Reg::Dma6BlockControl => (0x1F80_10E4, 4),
            Reg::Dma6Control => (0x1F80_10E8, 4),

            Reg::DmaControl => (0x1F80_10F0, 4),
            Reg::DmaInterrupt => (0x1F80_10F4, 4),

            // Timers
            Reg::Timer0Value => (0x1F80_1100, 4),
            Reg::Timer0Mode => (0x1F80_1104, 4),
            Reg::Timer0Target => (0x1F80_1108, 4),
            Reg::Timer1Value => (0x1F80_1110, 4),
            Reg::Timer1Mode => (0x1F80_1114, 4),
            Reg::Timer1Target => (0x1F80_1118, 4),
            Reg::Timer2Value => (0x1F80_1120, 4),
            Reg::Timer2Mode => (0x1F80_1124, 4),
            Reg::Timer2Target => (0x1F80_1128, 4),

            // CDROM
            Reg::Cdrom0 => (0x1F80_1800, 1),
            Reg::Cdrom1 => (0x1F80_1801, 1),
            Reg::Cdrom2 => (0x1F80_1802, 1),
            Reg::Cdrom3 => (0x1F80_1803, 1),

            // GPU
            Reg::Gp0 => (0x1F80_1810, 4),
            Reg::Gp1 => (0x1F80_1814, 4),

            // MDEC
            Reg::MdecCommand => (0x1F80_1820, 4),
            Reg::MdecStatus => (0x1F80_1824, 4),

            // SPU Voice {{{
            Reg::Voice0Volume => (0x1F80_1C00, 4),
            Reg::Voice0SampleRate => (0x1F80_1C04, 2),
            Reg::Voice0Start => (0x1F80_1C06, 2),
            Reg::Voice0ADSR => (0x1F80_1C08, 4),
            Reg::Voice0ADSRVolume => (0x1F80_1C0C, 2),
            Reg::Voice0Repeat => (0x1F80_1C0E, 2),

            Reg::Voice1Volume => (0x1F80_1C10, 4),
            Reg::Voice1SampleRate => (0x1F80_1C14, 2),
            Reg::Voice1Start => (0x1F80_1C16, 2),
            Reg::Voice1ADSR => (0x1F80_1C18, 4),
            Reg::Voice1ADSRVolume => (0x1F80_1C1C, 2),
            Reg::Voice1Repeat => (0x1F80_1C1E, 2),

            Reg::Voice2Volume => (0x1F80_1C20, 4),
            Reg::Voice2SampleRate => (0x1F80_1C24, 2),
            Reg::Voice2Start => (0x1F80_1C26, 2),
            Reg::Voice2ADSR => (0x1F80_1C28, 4),
            Reg::Voice2ADSRVolume => (0x1F80_1C2C, 2),
            Reg::Voice2Repeat => (0x1F80_1C2E, 2),

            Reg::Voice3Volume => (0x1F80_1C30, 4),
            Reg::Voice3SampleRate => (0x1F80_1C34, 2),
            Reg::Voice3Start => (0x1F80_1C36, 2),
            Reg::Voice3ADSR => (0x1F80_1C38, 4),
            Reg::Voice3ADSRVolume => (0x1F80_1C3C, 2),
            Reg::Voice3Repeat => (0x1F80_1C3E, 2),

            Reg::Voice4Volume => (0x1F80_1C40, 4),
            Reg::Voice4SampleRate => (0x1F80_1C44, 2),
            Reg::Voice4Start => (0x1F80_1C46, 2),
            Reg::Voice4ADSR => (0x1F80_1C48, 4),
            Reg::Voice4ADSRVolume => (0x1F80_1C4C, 2),
            Reg::Voice4Repeat => (0x1F80_1C4E, 2),

            Reg::Voice5Volume => (0x1F80_1C50, 4),
            Reg::Voice5SampleRate => (0x1F80_1C54, 2),
            Reg::Voice5Start => (0x1F80_1C56, 2),
            Reg::Voice5ADSR => (0x1F80_1C58, 4),
            Reg::Voice5ADSRVolume => (0x1F80_1C5C, 2),
            Reg::Voice5Repeat => (0x1F80_1C5E, 2),

            Reg::Voice6Volume => (0x1F80_1C60, 4),
            Reg::Voice6SampleRate => (0x1F80_1C64, 2),
            Reg::Voice6Start => (0x1F80_1C66, 2),
            Reg::Voice6ADSR => (0x1F80_1C68, 4),
            Reg::Voice6ADSRVolume => (0x1F80_1C6C, 2),
            Reg::Voice6Repeat => (0x1F80_1C6E, 2),

            Reg::Voice7Volume => (0x1F80_1C70, 4),
            Reg::Voice7SampleRate => (0x1F80_1C74, 2),
            Reg::Voice7Start => (0x1F80_1C76, 2),
            Reg::Voice7ADSR => (0x1F80_1C78, 4),
            Reg::Voice7ADSRVolume => (0x1F80_1C7C, 2),
            Reg::Voice7Repeat => (0x1F80_1C7E, 2),

            Reg::Voice8Volume => (0x1F80_1C80, 4),
            Reg::Voice8SampleRate => (0x1F80_1C84, 2),
            Reg::Voice8Start => (0x1F80_1C86, 2),
            Reg::Voice8ADSR => (0x1F80_1C88, 4),
            Reg::Voice8ADSRVolume => (0x1F80_1C8C, 2),
            Reg::Voice8Repeat => (0x1F80_1C8E, 2),

            Reg::Voice9Volume => (0x1F80_1C90, 4),
            Reg::Voice9SampleRate => (0x1F80_1C94, 2),
            Reg::Voice9Start => (0x1F80_1C96, 2),
            Reg::Voice9ADSR => (0x1F80_1C98, 4),
            Reg::Voice9ADSRVolume => (0x1F80_1C9C, 2),
            Reg::Voice9Repeat => (0x1F80_1C9E, 2),

            Reg::Voice10Volume => (0x1F80_1CA0, 4),
            Reg::Voice10SampleRate => (0x1F80_1CA4, 2),
            Reg::Voice10Start => (0x1F80_1CA6, 2),
            Reg::Voice10ADSR => (0x1F80_1CA8, 4),
            Reg::Voice10ADSRVolume => (0x1F80_1CAC, 2),
            Reg::Voice10Repeat => (0x1F80_1CAE, 2),

            Reg::Voice11Volume => (0x1F80_1CB0, 4),
            Reg::Voice11SampleRate => (0x1F80_1CB4, 2),
            Reg::Voice11Start => (0x1F80_1CB6, 2),
            Reg::Voice11ADSR => (0x1F80_1CB8, 4),
            Reg::Voice11ADSRVolume => (0x1F80_1CBC, 2),
            Reg::Voice11Repeat => (0x1F80_1CBE, 2),

            Reg::Voice12Volume => (0x1F80_1CC0, 4),
            Reg::Voice12SampleRate => (0x1F80_1CC4, 2),
            Reg::Voice12Start => (0x1F80_1CC6, 2),
            Reg::Voice12ADSR => (0x1F80_1CC8, 4),
            Reg::Voice12ADSRVolume => (0x1F80_1CCC, 2),
            Reg::Voice12Repeat => (0x1F80_1CCE, 2),

            Reg::Voice13Volume => (0x1F80_1CD0, 4),
            Reg::Voice13SampleRate => (0x1F80_1CD4, 2),
            Reg::Voice13Start => (0x1F80_1CD6, 2),
            Reg::Voice13ADSR => (0x1F80_1CD8, 4),
            Reg::Voice13ADSRVolume => (0x1F80_1CDC, 2),
            Reg::Voice13Repeat => (0x1F80_1CDE, 2),

            Reg::Voice14Volume => (0x1F80_1CE0, 4),
            Reg::Voice14SampleRate => (0x1F80_1CE4, 2),
            Reg::Voice14Start => (0x1F80_1CE6, 2),
            Reg::Voice14ADSR => (0x1F80_1CE8, 4),
            Reg::Voice14ADSRVolume => (0x1F80_1CEC, 2),
            Reg::Voice14Repeat => (0x1F80_1CEE, 2),

            Reg::Voice15Volume => (0x1F80_1CF0, 4),
            Reg::Voice15SampleRate => (0x1F80_1CF4, 2),
            Reg::Voice15Start => (0x1F80_1CF6, 2),
            Reg::Voice15ADSR => (0x1F80_1CF8, 4),
            Reg::Voice15ADSRVolume => (0x1F80_1CFC, 2),
            Reg::Voice15Repeat => (0x1F80_1CFE, 2),

            Reg::Voice16Volume => (0x1F80_1D00, 4),
            Reg::Voice16SampleRate => (0x1F80_1D04, 2),
            Reg::Voice16Start => (0x1F80_1D06, 2),
            Reg::Voice16ADSR => (0x1F80_1D08, 4),
            Reg::Voice16ADSRVolume => (0x1F80_1D0C, 2),
            Reg::Voice16Repeat => (0x1F80_1D0E, 2),

            Reg::Voice17Volume => (0x1F80_1D10, 4),
            Reg::Voice17SampleRate => (0x1F80_1D14, 2),
            Reg::Voice17Start => (0x1F80_1D16, 2),
            Reg::Voice17ADSR => (0x1F80_1D18, 4),
            Reg::Voice17ADSRVolume => (0x1F80_1D1C, 2),
            Reg::Voice17Repeat => (0x1F80_1D1E, 2),

            Reg::Voice18Volume => (0x1F80_1D20, 4),
            Reg::Voice18SampleRate => (0x1F80_1D24, 2),
            Reg::Voice18Start => (0x1F80_1D26, 2),
            Reg::Voice18ADSR => (0x1F80_1D28, 4),
            Reg::Voice18ADSRVolume => (0x1F80_1D2C, 2),
            Reg::Voice18Repeat => (0x1F80_1D2E, 2),

            Reg::Voice19Volume => (0x1F80_1D30, 4),
            Reg::Voice19SampleRate => (0x1F80_1D34, 2),
            Reg::Voice19Start => (0x1F80_1D36, 2),
            Reg::Voice19ADSR => (0x1F80_1D38, 4),
            Reg::Voice19ADSRVolume => (0x1F80_1D3C, 2),
            Reg::Voice19Repeat => (0x1F80_1D3E, 2),

            Reg::Voice20Volume => (0x1F80_1D40, 4),
            Reg::Voice20SampleRate => (0x1F80_1D44, 2),
            Reg::Voice20Start => (0x1F80_1D46, 2),
            Reg::Voice20ADSR => (0x1F80_1D48, 4),
            Reg::Voice20ADSRVolume => (0x1F80_1D4C, 2),
            Reg::Voice20Repeat => (0x1F80_1D4E, 2),

            Reg::Voice21Volume => (0x1F80_1D50, 4),
            Reg::Voice21SampleRate => (0x1F80_1D54, 2),
            Reg::Voice21Start => (0x1F80_1D56, 2),
            Reg::Voice21ADSR => (0x1F80_1D58, 4),
            Reg::Voice21ADSRVolume => (0x1F80_1D5C, 2),
            Reg::Voice21Repeat => (0x1F80_1D5E, 2),

            Reg::Voice22Volume => (0x1F80_1D60, 4),
            Reg::Voice22SampleRate => (0x1F80_1D64, 2),
            Reg::Voice22Start => (0x1F80_1D66, 2),
            Reg::Voice22ADSR => (0x1F80_1D68, 4),
            Reg::Voice22ADSRVolume => (0x1F80_1D6C, 2),
            Reg::Voice22Repeat => (0x1F80_1D6E, 2),

            Reg::Voice23Volume => (0x1F80_1D70, 4),
            Reg::Voice23SampleRate => (0x1F80_1D74, 2),
            Reg::Voice23Start => (0x1F80_1D76, 2),
            Reg::Voice23ADSR => (0x1F80_1D78, 4),
            Reg::Voice23ADSRVolume => (0x1F80_1D7C, 2),
            Reg::Voice23Repeat => (0x1F80_1D7E, 2),
            // }}}

            // SPU
            Reg::MainVolume => (0x1F80_1D80, 4),
            Reg::ReverbVolume => (0x1F80_1D84, 4),

            Reg::VoiceKeyOn => (0x1F80_1D88, 4),
            Reg::VoiceKeyOff => (0x1F80_1D8C, 4),
            Reg::VoiceChannelFmMode => (0x1F80_1D90, 4),
            Reg::VoiceChannelNoiseMode => (0x1F80_1D94, 4),
            Reg::VoiceChannelReverbMode => (0x1F80_1D98, 4),
            Reg::VoiceChannelEnabled => (0x1F80_1D9C, 4),

            Reg::SramReverbAddress => (0x1F80_1DA2, 2),
            Reg::SramInterruptAddress => (0x1F80_1DA4, 2),
            Reg::SramAddress => (0x1F80_1DA6, 2),
            Reg::SramFifo => (0x1F80_1DA8, 2),
            Reg::SpuControl => (0x1F80_1DAA, 2),
            Reg::SramControl => (0x1F80_1DAC, 2),
            Reg::SpuStatus => (0x1F80_1DAE, 2),

            Reg::CdVolume => (0x1F80_1DB0, 4),
            Reg::ExternVolume => (0x1F80_1DB4, 4),

            // Expansion Region 2
            Reg::Post => (0x1F80_2041, 1),
        };

        (PhysicalAddress(addr), width)
    }

    /// Returns the address of this register.
    #[inline(always)]
    pub const fn address(self) -> PhysicalAddress {
        self.address_and_width().0
    }

    /// Returns the width of this register.
    #[inline(always)]
    pub const fn width(self) -> usize {
        self.address_and_width().1
    }

    /// Returns the offset of the given address with respect to this register, but only if it's
    /// contained inside the register's range `addr..(addr + width)`.
    #[inline(always)]
    pub fn offset(self, addr: Address) -> Option<usize> {
        let (reg_addr, width) = self.address_and_width();
        addr.physical()?
            .value()
            .checked_sub(reg_addr.value())
            .and_then(|offset| ((offset as usize) < width).then_some(offset as usize))
    }

    pub fn is_spu_voice(&self) -> bool {
        (Reg::Voice0Volume.address()..=Reg::Voice23Repeat.address()).contains(&self.address())
    }

    pub fn dma_channel(&self) -> Option<dma::Channel> {
        Some(match self {
            Reg::Dma0Base | Reg::Dma0BlockControl | Reg::Dma0Control => dma::Channel::MdecIn,
            Reg::Dma1Base | Reg::Dma1BlockControl | Reg::Dma1Control => dma::Channel::MdecOut,
            Reg::Dma2Base | Reg::Dma2BlockControl | Reg::Dma2Control => dma::Channel::GPU,
            Reg::Dma3Base | Reg::Dma3BlockControl | Reg::Dma3Control => dma::Channel::CDROM,
            Reg::Dma4Base | Reg::Dma4BlockControl | Reg::Dma4Control => dma::Channel::SPU,
            Reg::Dma5Base | Reg::Dma5BlockControl | Reg::Dma5Control => dma::Channel::PIO,
            Reg::Dma6Base | Reg::Dma6BlockControl | Reg::Dma6Control => dma::Channel::OTC,
            _ => return None,
        })
    }

    pub fn cdrom_reg(&self) -> Option<cdrom::Reg> {
        Some(match self {
            Reg::Cdrom0 => cdrom::Reg::Reg0,
            Reg::Cdrom1 => cdrom::Reg::Reg1,
            Reg::Cdrom2 => cdrom::Reg::Reg2,
            Reg::Cdrom3 => cdrom::Reg::Reg3,
            _ => return None,
        })
    }

    /// Returns the register for which a given address in inside, if any, and the offset of the
    /// address.
    pub fn reg_and_offset(addr: Address) -> Option<(Reg, usize)> {
        for reg in Self::VARIANTS {
            if let Some(offset) = reg.offset(addr) {
                return Some((*reg, offset));
            }
        }

        None
    }
}
