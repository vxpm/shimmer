//! Items related to memory mapped IO.

use super::{Address, PhysicalAddress};
use crate::{cdrom, dma};
use strum::VariantArray;

/// A memory mapped register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, VariantArray)]
pub enum Reg {
    // Memory Control 1
    Expansion1Base = 0x1F80_1000,
    Expansion2Base = 0x1F80_1004,
    Expansion1Delay = 0x1F80_1008,
    Expansion3Delay = 0x1F80_100C,
    BiosDelay = 0x1F80_1010,
    SpuDelay = 0x1F80_1014,
    CdromDelay = 0x1F80_1018,
    Expansion2Delay = 0x1F80_101C,
    CommonDelay = 0x1F80_1020,

    // Peripheral IO
    JoyData = 0x1F80_1040,
    JoyStat = 0x1F80_1044,
    JoyMode = 0x1F80_1048,
    JoyControl = 0x1F80_104A,
    JoyBaud = 0x1F80_104E,

    // Memory Control 2
    RamSize = 0x1F80_1060,

    // Interrupt
    InterruptStatus = 0x1F80_1070,
    InterruptMask = 0x1F80_1074,

    // DMA
    Dma0Base = 0x1F80_1080,
    Dma0BlockControl = 0x1F80_1084,
    Dma0Control = 0x1F80_1088,

    Dma1Base = 0x1F80_1090,
    Dma1BlockControl = 0x1F80_1094,
    Dma1Control = 0x1F80_1098,

    Dma2Base = 0x1F80_10A0,
    Dma2BlockControl = 0x1F80_10A4,
    Dma2Control = 0x1F80_10A8,

    Dma3Base = 0x1F80_10B0,
    Dma3BlockControl = 0x1F80_10B4,
    Dma3Control = 0x1F80_10B8,

    Dma4Base = 0x1F80_10C0,
    Dma4BlockControl = 0x1F80_10C4,
    Dma4Control = 0x1F80_10C8,

    Dma5Base = 0x1F80_10D0,
    Dma5BlockControl = 0x1F80_10D4,
    Dma5Control = 0x1F80_10D8,

    Dma6Base = 0x1F80_10E0,
    Dma6BlockControl = 0x1F80_10E4,
    Dma6Control = 0x1F80_10E8,

    DmaControl = 0x1F80_10F0,
    DmaInterrupt = 0x1F80_10F4,

    // Timers
    Timer0Value = 0x1F80_1100,
    Timer0Mode = 0x1F80_1104,
    Timer0Target = 0x1F80_1108,
    Timer1Value = 0x1F80_1110,
    Timer1Mode = 0x1F80_1114,
    Timer1Target = 0x1F80_1118,
    Timer2Value = 0x1F80_1120,
    Timer2Mode = 0x1F80_1124,
    Timer2Target = 0x1F80_1128,

    // CDROM
    Cdrom0 = 0x1F80_1800,
    Cdrom1 = 0x1F80_1801,
    Cdrom2 = 0x1F80_1802,
    Cdrom3 = 0x1F80_1803,

    // GPU
    Gp0 = 0x1F80_1810,
    Gp1 = 0x1F80_1814,

    // MDEC
    MdecCommand = 0x1F80_1820,
    MdecStatus = 0x1F80_1824,

    // SPU Voice {{{
    Voice0Volume = 0x1F80_1C00,
    Voice0SampleRate = 0x1F80_1C04,
    Voice0Start = 0x1F80_1C06,
    Voice0ADSR = 0x1F80_1C08,
    Voice0ADSRVolume = 0x1F80_1C0C,
    Voice0Repeat = 0x1F80_1C0E,

    Voice1Volume = 0x1F80_1C10,
    Voice1SampleRate = 0x1F80_1C14,
    Voice1Start = 0x1F80_1C16,
    Voice1ADSR = 0x1F80_1C18,
    Voice1ADSRVolume = 0x1F80_1C1C,
    Voice1Repeat = 0x1F80_1C1E,

    Voice2Volume = 0x1F80_1C20,
    Voice2SampleRate = 0x1F80_1C24,
    Voice2Start = 0x1F80_1C26,
    Voice2ADSR = 0x1F80_1C28,
    Voice2ADSRVolume = 0x1F80_1C2C,
    Voice2Repeat = 0x1F80_1C2E,

    Voice3Volume = 0x1F80_1C30,
    Voice3SampleRate = 0x1F80_1C34,
    Voice3Start = 0x1F80_1C36,
    Voice3ADSR = 0x1F80_1C38,
    Voice3ADSRVolume = 0x1F80_1C3C,
    Voice3Repeat = 0x1F80_1C3E,

    Voice4Volume = 0x1F80_1C40,
    Voice4SampleRate = 0x1F80_1C44,
    Voice4Start = 0x1F80_1C46,
    Voice4ADSR = 0x1F80_1C48,
    Voice4ADSRVolume = 0x1F80_1C4C,
    Voice4Repeat = 0x1F80_1C4E,

    Voice5Volume = 0x1F80_1C50,
    Voice5SampleRate = 0x1F80_1C54,
    Voice5Start = 0x1F80_1C56,
    Voice5ADSR = 0x1F80_1C58,
    Voice5ADSRVolume = 0x1F80_1C5C,
    Voice5Repeat = 0x1F80_1C5E,

    Voice6Volume = 0x1F80_1C60,
    Voice6SampleRate = 0x1F80_1C64,
    Voice6Start = 0x1F80_1C66,
    Voice6ADSR = 0x1F80_1C68,
    Voice6ADSRVolume = 0x1F80_1C6C,
    Voice6Repeat = 0x1F80_1C6E,

    Voice7Volume = 0x1F80_1C70,
    Voice7SampleRate = 0x1F80_1C74,
    Voice7Start = 0x1F80_1C76,
    Voice7ADSR = 0x1F80_1C78,
    Voice7ADSRVolume = 0x1F80_1C7C,
    Voice7Repeat = 0x1F80_1C7E,

    Voice8Volume = 0x1F80_1C80,
    Voice8SampleRate = 0x1F80_1C84,
    Voice8Start = 0x1F80_1C86,
    Voice8ADSR = 0x1F80_1C88,
    Voice8ADSRVolume = 0x1F80_1C8C,
    Voice8Repeat = 0x1F80_1C8E,

    Voice9Volume = 0x1F80_1C90,
    Voice9SampleRate = 0x1F80_1C94,
    Voice9Start = 0x1F80_1C96,
    Voice9ADSR = 0x1F80_1C98,
    Voice9ADSRVolume = 0x1F80_1C9C,
    Voice9Repeat = 0x1F80_1C9E,

    Voice10Volume = 0x1F80_1CA0,
    Voice10SampleRate = 0x1F80_1CA4,
    Voice10Start = 0x1F80_1CA6,
    Voice10ADSR = 0x1F80_1CA8,
    Voice10ADSRVolume = 0x1F80_1CAC,
    Voice10Repeat = 0x1F80_1CAE,

    Voice11Volume = 0x1F80_1CB0,
    Voice11SampleRate = 0x1F80_1CB4,
    Voice11Start = 0x1F80_1CB6,
    Voice11ADSR = 0x1F80_1CB8,
    Voice11ADSRVolume = 0x1F80_1CBC,
    Voice11Repeat = 0x1F80_1CBE,

    Voice12Volume = 0x1F80_1CC0,
    Voice12SampleRate = 0x1F80_1CC4,
    Voice12Start = 0x1F80_1CC6,
    Voice12ADSR = 0x1F80_1CC8,
    Voice12ADSRVolume = 0x1F80_1CCC,
    Voice12Repeat = 0x1F80_1CCE,

    Voice13Volume = 0x1F80_1CD0,
    Voice13SampleRate = 0x1F80_1CD4,
    Voice13Start = 0x1F80_1CD6,
    Voice13ADSR = 0x1F80_1CD8,
    Voice13ADSRVolume = 0x1F80_1CDC,
    Voice13Repeat = 0x1F80_1CDE,

    Voice14Volume = 0x1F80_1CE0,
    Voice14SampleRate = 0x1F80_1CE4,
    Voice14Start = 0x1F80_1CE6,
    Voice14ADSR = 0x1F80_1CE8,
    Voice14ADSRVolume = 0x1F80_1CEC,
    Voice14Repeat = 0x1F80_1CEE,

    Voice15Volume = 0x1F80_1CF0,
    Voice15SampleRate = 0x1F80_1CF4,
    Voice15Start = 0x1F80_1CF6,
    Voice15ADSR = 0x1F80_1CF8,
    Voice15ADSRVolume = 0x1F80_1CFC,
    Voice15Repeat = 0x1F80_1CFE,

    Voice16Volume = 0x1F80_1D00,
    Voice16SampleRate = 0x1F80_1D04,
    Voice16Start = 0x1F80_1D06,
    Voice16ADSR = 0x1F80_1D08,
    Voice16ADSRVolume = 0x1F80_1D0C,
    Voice16Repeat = 0x1F80_1D0E,

    Voice17Volume = 0x1F80_1D10,
    Voice17SampleRate = 0x1F80_1D14,
    Voice17Start = 0x1F80_1D16,
    Voice17ADSR = 0x1F80_1D18,
    Voice17ADSRVolume = 0x1F80_1D1C,
    Voice17Repeat = 0x1F80_1D1E,

    Voice18Volume = 0x1F80_1D20,
    Voice18SampleRate = 0x1F80_1D24,
    Voice18Start = 0x1F80_1D26,
    Voice18ADSR = 0x1F80_1D28,
    Voice18ADSRVolume = 0x1F80_1D2C,
    Voice18Repeat = 0x1F80_1D2E,

    Voice19Volume = 0x1F80_1D30,
    Voice19SampleRate = 0x1F80_1D34,
    Voice19Start = 0x1F80_1D36,
    Voice19ADSR = 0x1F80_1D38,
    Voice19ADSRVolume = 0x1F80_1D3C,
    Voice19Repeat = 0x1F80_1D3E,

    Voice20Volume = 0x1F80_1D40,
    Voice20SampleRate = 0x1F80_1D44,
    Voice20Start = 0x1F80_1D46,
    Voice20ADSR = 0x1F80_1D48,
    Voice20ADSRVolume = 0x1F80_1D4C,
    Voice20Repeat = 0x1F80_1D4E,

    Voice21Volume = 0x1F80_1D50,
    Voice21SampleRate = 0x1F80_1D54,
    Voice21Start = 0x1F80_1D56,
    Voice21ADSR = 0x1F80_1D58,
    Voice21ADSRVolume = 0x1F80_1D5C,
    Voice21Repeat = 0x1F80_1D5E,

    Voice22Volume = 0x1F80_1D60,
    Voice22SampleRate = 0x1F80_1D64,
    Voice22Start = 0x1F80_1D66,
    Voice22ADSR = 0x1F80_1D68,
    Voice22ADSRVolume = 0x1F80_1D6C,
    Voice22Repeat = 0x1F80_1D6E,

    Voice23Volume = 0x1F80_1D70,
    Voice23SampleRate = 0x1F80_1D74,
    Voice23Start = 0x1F80_1D76,
    Voice23ADSR = 0x1F80_1D78,
    Voice23ADSRVolume = 0x1F80_1D7C,
    Voice23Repeat = 0x1F80_1D7E,
    // }}}

    // SPU
    MainVolume = 0x1F80_1D80,
    ReverbVolume = 0x1F80_1D84,

    VoiceKeyOn = 0x1F80_1D88,
    VoiceKeyOff = 0x1F80_1D8C,
    VoiceChannelFmMode = 0x1F80_1D90,
    VoiceChannelNoiseMode = 0x1F80_1D94,
    VoiceChannelReverbMode = 0x1F80_1D98,
    VoiceChannelEnabled = 0x1F80_1D9C,

    SramReverbAddress = 0x1F80_1DA2,
    SramInterruptAddress = 0x1F80_1DA4,
    SramAddress = 0x1F80_1DA6,
    SramFifo = 0x1F80_1DA8,
    SpuControl = 0x1F80_1DAA,
    SramControl = 0x1F80_1DAC,
    SpuStatus = 0x1F80_1DAE,

    CdVolume = 0x1F80_1DB0,
    ExternVolume = 0x1F80_1DB4,

    // Expansion Region 2
    Post = 0x1F80_2041,
}

impl Reg {
    /// Returns the address of this register.
    #[inline(always)]
    pub const fn address(self) -> PhysicalAddress {
        PhysicalAddress(self as u32)
    }

    /// Returns the width of this register.
    pub const fn width(self) -> usize {
        match self {
            // Memory Control 1
            Reg::Expansion1Base => 4,
            Reg::Expansion2Base => 4,
            Reg::Expansion1Delay => 4,
            Reg::Expansion3Delay => 4,
            Reg::BiosDelay => 4,
            Reg::SpuDelay => 4,
            Reg::CdromDelay => 4,
            Reg::Expansion2Delay => 4,
            Reg::CommonDelay => 4,

            // Peripheral IO
            Reg::JoyData => 4,
            Reg::JoyStat => 4,
            Reg::JoyMode => 2,
            Reg::JoyControl => 2,
            Reg::JoyBaud => 2,

            // Memory Control 2
            Reg::RamSize => 4,

            // Interrupt
            Reg::InterruptStatus => 4,
            Reg::InterruptMask => 4,

            // DMA
            Reg::Dma0Base => 4,
            Reg::Dma0BlockControl => 4,
            Reg::Dma0Control => 4,

            Reg::Dma1Base => 4,
            Reg::Dma1BlockControl => 4,
            Reg::Dma1Control => 4,

            Reg::Dma2Base => 4,
            Reg::Dma2BlockControl => 4,
            Reg::Dma2Control => 4,

            Reg::Dma3Base => 4,
            Reg::Dma3BlockControl => 4,
            Reg::Dma3Control => 4,

            Reg::Dma4Base => 4,
            Reg::Dma4BlockControl => 4,
            Reg::Dma4Control => 4,

            Reg::Dma5Base => 4,
            Reg::Dma5BlockControl => 4,
            Reg::Dma5Control => 4,

            Reg::Dma6Base => 4,
            Reg::Dma6BlockControl => 4,
            Reg::Dma6Control => 4,

            Reg::DmaControl => 4,
            Reg::DmaInterrupt => 4,

            // Timers
            Reg::Timer0Value => 4,
            Reg::Timer0Mode => 4,
            Reg::Timer0Target => 4,
            Reg::Timer1Value => 4,
            Reg::Timer1Mode => 4,
            Reg::Timer1Target => 4,
            Reg::Timer2Value => 4,
            Reg::Timer2Mode => 4,
            Reg::Timer2Target => 4,

            // CDROM
            Reg::Cdrom0 => 1,
            Reg::Cdrom1 => 1,
            Reg::Cdrom2 => 1,
            Reg::Cdrom3 => 1,

            // GPU
            Reg::Gp0 => 4,
            Reg::Gp1 => 4,

            // MDEC
            Reg::MdecCommand => 4,
            Reg::MdecStatus => 4,

            // SPU Voice {{{
            Reg::Voice0Volume => 4,
            Reg::Voice0SampleRate => 2,
            Reg::Voice0Start => 2,
            Reg::Voice0ADSR => 4,
            Reg::Voice0ADSRVolume => 2,
            Reg::Voice0Repeat => 2,

            Reg::Voice1Volume => 4,
            Reg::Voice1SampleRate => 2,
            Reg::Voice1Start => 2,
            Reg::Voice1ADSR => 4,
            Reg::Voice1ADSRVolume => 2,
            Reg::Voice1Repeat => 2,

            Reg::Voice2Volume => 4,
            Reg::Voice2SampleRate => 2,
            Reg::Voice2Start => 2,
            Reg::Voice2ADSR => 4,
            Reg::Voice2ADSRVolume => 2,
            Reg::Voice2Repeat => 2,

            Reg::Voice3Volume => 4,
            Reg::Voice3SampleRate => 2,
            Reg::Voice3Start => 2,
            Reg::Voice3ADSR => 4,
            Reg::Voice3ADSRVolume => 2,
            Reg::Voice3Repeat => 2,

            Reg::Voice4Volume => 4,
            Reg::Voice4SampleRate => 2,
            Reg::Voice4Start => 2,
            Reg::Voice4ADSR => 4,
            Reg::Voice4ADSRVolume => 2,
            Reg::Voice4Repeat => 2,

            Reg::Voice5Volume => 4,
            Reg::Voice5SampleRate => 2,
            Reg::Voice5Start => 2,
            Reg::Voice5ADSR => 4,
            Reg::Voice5ADSRVolume => 2,
            Reg::Voice5Repeat => 2,

            Reg::Voice6Volume => 4,
            Reg::Voice6SampleRate => 2,
            Reg::Voice6Start => 2,
            Reg::Voice6ADSR => 4,
            Reg::Voice6ADSRVolume => 2,
            Reg::Voice6Repeat => 2,

            Reg::Voice7Volume => 4,
            Reg::Voice7SampleRate => 2,
            Reg::Voice7Start => 2,
            Reg::Voice7ADSR => 4,
            Reg::Voice7ADSRVolume => 2,
            Reg::Voice7Repeat => 2,

            Reg::Voice8Volume => 4,
            Reg::Voice8SampleRate => 2,
            Reg::Voice8Start => 2,
            Reg::Voice8ADSR => 4,
            Reg::Voice8ADSRVolume => 2,
            Reg::Voice8Repeat => 2,

            Reg::Voice9Volume => 4,
            Reg::Voice9SampleRate => 2,
            Reg::Voice9Start => 2,
            Reg::Voice9ADSR => 4,
            Reg::Voice9ADSRVolume => 2,
            Reg::Voice9Repeat => 2,

            Reg::Voice10Volume => 4,
            Reg::Voice10SampleRate => 2,
            Reg::Voice10Start => 2,
            Reg::Voice10ADSR => 4,
            Reg::Voice10ADSRVolume => 2,
            Reg::Voice10Repeat => 2,

            Reg::Voice11Volume => 4,
            Reg::Voice11SampleRate => 2,
            Reg::Voice11Start => 2,
            Reg::Voice11ADSR => 4,
            Reg::Voice11ADSRVolume => 2,
            Reg::Voice11Repeat => 2,

            Reg::Voice12Volume => 4,
            Reg::Voice12SampleRate => 2,
            Reg::Voice12Start => 2,
            Reg::Voice12ADSR => 4,
            Reg::Voice12ADSRVolume => 2,
            Reg::Voice12Repeat => 2,

            Reg::Voice13Volume => 4,
            Reg::Voice13SampleRate => 2,
            Reg::Voice13Start => 2,
            Reg::Voice13ADSR => 4,
            Reg::Voice13ADSRVolume => 2,
            Reg::Voice13Repeat => 2,

            Reg::Voice14Volume => 4,
            Reg::Voice14SampleRate => 2,
            Reg::Voice14Start => 2,
            Reg::Voice14ADSR => 4,
            Reg::Voice14ADSRVolume => 2,
            Reg::Voice14Repeat => 2,

            Reg::Voice15Volume => 4,
            Reg::Voice15SampleRate => 2,
            Reg::Voice15Start => 2,
            Reg::Voice15ADSR => 4,
            Reg::Voice15ADSRVolume => 2,
            Reg::Voice15Repeat => 2,

            Reg::Voice16Volume => 4,
            Reg::Voice16SampleRate => 2,
            Reg::Voice16Start => 2,
            Reg::Voice16ADSR => 4,
            Reg::Voice16ADSRVolume => 2,
            Reg::Voice16Repeat => 2,

            Reg::Voice17Volume => 4,
            Reg::Voice17SampleRate => 2,
            Reg::Voice17Start => 2,
            Reg::Voice17ADSR => 4,
            Reg::Voice17ADSRVolume => 2,
            Reg::Voice17Repeat => 2,

            Reg::Voice18Volume => 4,
            Reg::Voice18SampleRate => 2,
            Reg::Voice18Start => 2,
            Reg::Voice18ADSR => 4,
            Reg::Voice18ADSRVolume => 2,
            Reg::Voice18Repeat => 2,

            Reg::Voice19Volume => 4,
            Reg::Voice19SampleRate => 2,
            Reg::Voice19Start => 2,
            Reg::Voice19ADSR => 4,
            Reg::Voice19ADSRVolume => 2,
            Reg::Voice19Repeat => 2,

            Reg::Voice20Volume => 4,
            Reg::Voice20SampleRate => 2,
            Reg::Voice20Start => 2,
            Reg::Voice20ADSR => 4,
            Reg::Voice20ADSRVolume => 2,
            Reg::Voice20Repeat => 2,

            Reg::Voice21Volume => 4,
            Reg::Voice21SampleRate => 2,
            Reg::Voice21Start => 2,
            Reg::Voice21ADSR => 4,
            Reg::Voice21ADSRVolume => 2,
            Reg::Voice21Repeat => 2,

            Reg::Voice22Volume => 4,
            Reg::Voice22SampleRate => 2,
            Reg::Voice22Start => 2,
            Reg::Voice22ADSR => 4,
            Reg::Voice22ADSRVolume => 2,
            Reg::Voice22Repeat => 2,

            Reg::Voice23Volume => 4,
            Reg::Voice23SampleRate => 2,
            Reg::Voice23Start => 2,
            Reg::Voice23ADSR => 4,
            Reg::Voice23ADSRVolume => 2,
            Reg::Voice23Repeat => 2,
            // }}}

            // SPU
            Reg::MainVolume => 4,
            Reg::ReverbVolume => 4,

            Reg::VoiceKeyOn => 4,
            Reg::VoiceKeyOff => 4,
            Reg::VoiceChannelFmMode => 4,
            Reg::VoiceChannelNoiseMode => 4,
            Reg::VoiceChannelReverbMode => 4,
            Reg::VoiceChannelEnabled => 4,

            Reg::SramReverbAddress => 2,
            Reg::SramInterruptAddress => 2,
            Reg::SramAddress => 2,
            Reg::SramFifo => 2,
            Reg::SpuControl => 2,
            Reg::SramControl => 2,
            Reg::SpuStatus => 2,

            Reg::CdVolume => 4,
            Reg::ExternVolume => 4,

            // Expansion Region 2
            Reg::Post => 1,
        }
    }

    /// Returns the offset of the given address with respect to this register, but only if it's
    /// contained inside the register's range `addr..(addr + width)`.
    #[inline(always)]
    pub fn offset(self, addr: Address) -> Option<usize> {
        let reg_addr = self.address();
        addr.physical()?
            .value()
            .checked_sub(reg_addr.value())
            .and_then(|offset| {
                // PERF: 4 is the maximum, check for it first before checking for the actual width
                let offset = offset as usize;
                (offset < 4 && offset < self.width()).then_some(offset)
            })
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
