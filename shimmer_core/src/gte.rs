pub mod fixed;
pub mod instr;

use bitos::{BitUtils, bitos};
use zerocopy::transmute_mut;

pub type Int44 = fixed::Integer<44>;

fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> (T, bool) {
    if value < min {
        (min, true)
    } else if value > max {
        (max, true)
    } else {
        (value, false)
    }
}

#[bitos(5)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DataReg {
    R0,
    R1,
    R2,
    R3,
    R4,
    R5,
    R6,
    R7,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
    R16,
    R17,
    R18,
    R19,
    R20,
    R21,
    R22,
    R23,
    R24,
    R25,
    R26,
    R27,
    R28,
    R29,
    R30,
    R31,
}

#[bitos(5)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ControlReg {
    R32,
    R33,
    R34,
    R35,
    R36,
    R37,
    R38,
    R39,
    R40,
    R41,
    R42,
    R43,
    R44,
    R45,
    R46,
    R47,
    R48,
    R49,
    R50,
    R51,
    R52,
    R53,
    R54,
    R55,
    R56,
    R57,
    R58,
    R59,
    R60,
    R61,
    R62,
    R63,
}

#[bitos(6)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Reg {
    R0,
    R1,
    R2,
    R3,
    R4,
    R5,
    R6,
    R7,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
    R16,
    R17,
    R18,
    R19,
    R20,
    R21,
    R22,
    R23,
    R24,
    R25,
    R26,
    R27,
    R28,
    R29,
    R30,
    R31,
    R32,
    R33,
    R34,
    R35,
    R36,
    R37,
    R38,
    R39,
    R40,
    R41,
    R42,
    R43,
    R44,
    R45,
    R46,
    R47,
    R48,
    R49,
    R50,
    R51,
    R52,
    R53,
    R54,
    R55,
    R56,
    R57,
    R58,
    R59,
    R60,
    R61,
    R62,
    R63,
}

impl Reg {
    pub const VXY0: Reg = Reg::R0;
    pub const VZ0: Reg = Reg::R1;
    pub const VXY1: Reg = Reg::R2;
    pub const VZ1: Reg = Reg::R3;
    pub const VXY2: Reg = Reg::R4;
    pub const VZ2: Reg = Reg::R5;

    pub const RGBC: Reg = Reg::R6;

    pub const OTZ: Reg = Reg::R7;
    pub const IR0: Reg = Reg::R8;
    pub const IR1: Reg = Reg::R9;
    pub const IR2: Reg = Reg::R10;
    pub const IR3: Reg = Reg::R11;

    pub const SXY0: Reg = Reg::R12;
    pub const SXY1: Reg = Reg::R13;
    pub const SXY2: Reg = Reg::R14;
    pub const SXYP: Reg = Reg::R15;

    pub const SZ0: Reg = Reg::R16;
    pub const SZ1: Reg = Reg::R17;
    pub const SZ2: Reg = Reg::R18;
    pub const SZ3: Reg = Reg::R19;

    pub const RGB0: Reg = Reg::R20;
    pub const RGB1: Reg = Reg::R21;
    pub const RGB2: Reg = Reg::R22;

    pub const MAC0: Reg = Reg::R24;
    pub const MAC1: Reg = Reg::R25;
    pub const MAC2: Reg = Reg::R26;
    pub const MAC3: Reg = Reg::R27;

    pub const IRGB: Reg = Reg::R28;
    pub const ORGB: Reg = Reg::R29;

    pub const LZCS: Reg = Reg::R30;
    pub const LZCR: Reg = Reg::R31;

    pub const RT_11_12: Reg = Reg::R32;
    pub const RT_13_21: Reg = Reg::R33;
    pub const RT_22_23: Reg = Reg::R34;
    pub const RT_31_32: Reg = Reg::R35;
    pub const RT_33_SS: Reg = Reg::R36;

    pub const TRX: Reg = Reg::R37;
    pub const TRY: Reg = Reg::R38;
    pub const TRZ: Reg = Reg::R39;

    pub const L_11_12: Reg = Reg::R40;
    pub const L_13_21: Reg = Reg::R41;
    pub const L_22_23: Reg = Reg::R42;
    pub const L_31_32: Reg = Reg::R43;
    pub const L_33_SS: Reg = Reg::R44;

    pub const BCR: Reg = Reg::R45;
    pub const BCG: Reg = Reg::R46;
    pub const BCB: Reg = Reg::R47;

    pub const L_R1_R2: Reg = Reg::R48;
    pub const L_R3_G1: Reg = Reg::R49;
    pub const L_G2_G3: Reg = Reg::R50;
    pub const L_B1_B2: Reg = Reg::R51;
    pub const L_B3_SS: Reg = Reg::R52;

    pub const FCR: Reg = Reg::R53;
    pub const FCG: Reg = Reg::R54;
    pub const FCB: Reg = Reg::R55;

    pub const OFX: Reg = Reg::R56;
    pub const OFY: Reg = Reg::R57;
    pub const H: Reg = Reg::R58;
    pub const DQA: Reg = Reg::R59;
    pub const DQB: Reg = Reg::R60;

    pub const ZSF3: Reg = Reg::R61;
    pub const ZSF4: Reg = Reg::R62;

    pub const FLAG: Reg = Reg::R63;
}

impl From<DataReg> for Reg {
    fn from(value: DataReg) -> Self {
        unsafe { std::mem::transmute(value) }
    }
}

impl From<ControlReg> for Reg {
    fn from(value: ControlReg) -> Self {
        unsafe { std::mem::transmute(value as u8 + 32) }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    ClampedIR0 = 12,
    ClampedSY2 = 13,
    ClampedSX2 = 14,
    UnderflowedMAC0 = 15,
    OverflowedMAC0 = 16,
    DivideOverflow = 17,
    ClampedZ = 18,
    ClampedB = 19,
    ClampedG = 20,
    ClampedR = 21,
    ClampedIR3 = 22,
    ClampedIR2 = 23,
    ClampedIR1 = 24,
    UnderflowedMAC3 = 25,
    UnderflowedMAC2 = 26,
    UnderflowedMAC1 = 27,
    OverflowedMAC3 = 28,
    OverflowedMAC2 = 29,
    OverflowedMAC1 = 30,
}

/// The registers of the GTE.
#[derive(Clone)]
pub struct Registers([u32; 64]);

impl Default for Registers {
    fn default() -> Self {
        Self([0u32; 64])
    }
}

impl Registers {
    #[inline(always)]
    pub fn read(&self, reg: Reg) -> u32 {
        match reg {
            Reg::H => self.0[reg as usize] as i16 as i32 as u32,
            Reg::FLAG => {
                let value = self.0[reg as usize];
                let err_flag = (value.bits(23, 31) | value.bits(13, 19)) > 0;
                value | if err_flag { 0x8000_0000 } else { 0 }
            }
            _ => self.0[reg as usize],
        }
    }

    #[inline(always)]
    pub fn write(&mut self, reg: Reg, value: u32) {
        match reg {
            Reg::VZ0 | Reg::VZ1 | Reg::VZ2 => self.0[reg as usize] = value as i16 as i32 as u32,
            Reg::OTZ => self.0[reg as usize] = value as u16 as u32,
            Reg::IR0 | Reg::IR1 | Reg::IR2 | Reg::IR3 => {
                self.0[reg as usize] = value as i16 as i32 as u32;
                self.update_irgb();
            }
            Reg::SXY2 => {
                self.0[Reg::SXY2 as usize] = value;
                self.0[Reg::SXYP as usize] = value;
            }
            Reg::SXYP => {
                self.0[Reg::SXY0 as usize] = self.0[Reg::SXY1 as usize];
                self.0[Reg::SXY1 as usize] = self.0[Reg::SXY2 as usize];
                self.0[Reg::SXY2 as usize] = value;
                self.0[Reg::SXYP as usize] = value;
            }
            Reg::SZ0 | Reg::SZ1 | Reg::SZ2 | Reg::SZ3 => self.0[reg as usize] = value as u16 as u32,
            Reg::IRGB => {
                self.0[Reg::IR1 as usize] = value.bits(0, 5) << 7;
                self.0[Reg::IR2 as usize] = value.bits(5, 10) << 7;
                self.0[Reg::IR3 as usize] = value.bits(10, 15) << 7;
                self.update_irgb();
            }
            Reg::ORGB => {
                // do nothing
            }
            Reg::LZCS => {
                self.0[reg as usize] = value;
                self.0[Reg::LZCR as usize] = if value as i32 >= 0 {
                    value.leading_zeros()
                } else {
                    value.leading_ones()
                };
            }
            Reg::LZCR => {
                // do nothing
            }
            Reg::RT_33_SS | Reg::L_33_SS | Reg::L_B3_SS => {
                self.0[reg as usize] = value as i16 as i32 as u32
            }
            Reg::DQA => self.0[reg as usize] = value as i16 as i32 as u32,
            Reg::ZSF3 | Reg::ZSF4 => self.0[reg as usize] = value as i16 as i32 as u32,
            Reg::FLAG => self.0[reg as usize] = value & !0x8000_0FFF,
            _ => self.0[reg as usize] = value,
        }
    }

    pub fn update_irgb(&mut self) {
        let (r, _) = clamp(self.0[Reg::IR1 as usize] as i32 / 0x80, 0, 0x1F);
        let (g, _) = clamp(self.0[Reg::IR2 as usize] as i32 / 0x80, 0, 0x1F);
        let (b, _) = clamp(self.0[Reg::IR3 as usize] as i32 / 0x80, 0, 0x1F);

        let value = 0
            .with_bits(0, 5, r as u32)
            .with_bits(5, 10, g as u32)
            .with_bits(10, 15, b as u32);

        self.0[Reg::IRGB as usize] = value;
        self.0[Reg::ORGB as usize] = value;
    }

    fn inner_set_ir<const N: usize>(&mut self, value: i32, no_neg: bool) -> i32 {
        const { assert!(N <= 3) };

        let lower = if no_neg { 0 } else { -0x8000 };
        let higher = if N == 0 { 0x1000 } else { 0x7FFF };

        let (clamped_value, clamped) = clamp(value, lower, higher);
        let (reg, flag) = match N {
            0 => (Reg::IR0, Flag::ClampedIR0),
            1 => (Reg::IR1, Flag::ClampedIR1),
            2 => (Reg::IR2, Flag::ClampedIR2),
            3 => (Reg::IR3, Flag::ClampedIR3),
            _ => unreachable!(),
        };

        self.write(reg, clamped_value as u32);
        self.merge_flag(flag, clamped);

        clamped_value
    }

    pub fn set_ir0(&mut self, value: i32) {
        self.inner_set_ir::<0>(value, true);
    }

    pub fn set_ir1(&mut self, value: i32, no_neg: bool) {
        self.inner_set_ir::<1>(value, no_neg);
    }

    pub fn set_ir2(&mut self, value: i32, no_neg: bool) {
        self.inner_set_ir::<2>(value, no_neg);
    }

    pub fn set_ir3(&mut self, value: i32, no_neg: bool) {
        self.inner_set_ir::<3>(value, no_neg);
    }

    fn inner_set_mac<const N: usize>(&mut self, value: Int44, shift: bool) -> i32 {
        const { assert!(N > 0 && N <= 3) };

        let shifted = if shift {
            value.value() >> 12
        } else {
            value.value()
        };

        let (reg, flag_underflow, flag_overflow) = match N {
            1 => (Reg::MAC1, Flag::UnderflowedMAC1, Flag::OverflowedMAC1),
            2 => (Reg::MAC2, Flag::UnderflowedMAC2, Flag::OverflowedMAC2),
            3 => (Reg::MAC3, Flag::UnderflowedMAC3, Flag::OverflowedMAC3),
            _ => unreachable!(),
        };

        self.write(reg, shifted as u32);
        self.merge_flag(flag_underflow, value.underflowed());
        self.merge_flag(flag_overflow, value.overflowed());

        shifted as i32
    }

    pub fn set_mac0(&mut self, value: Int44) -> i32 {
        let value = value.value();
        self.write(Reg::MAC0, value as u32);
        self.merge_flag(Flag::UnderflowedMAC0, value < -(1 << 31));
        self.merge_flag(Flag::OverflowedMAC0, value > (1 << 31));

        value as i32
    }

    pub fn set_mac1(&mut self, value: Int44, shift: bool) -> i32 {
        self.inner_set_mac::<1>(value, shift)
    }

    pub fn set_mac2(&mut self, value: Int44, shift: bool) -> i32 {
        self.inner_set_mac::<2>(value, shift)
    }

    pub fn set_mac3(&mut self, value: Int44, shift: bool) -> i32 {
        self.inner_set_mac::<3>(value, shift)
    }

    pub fn set_mac_ir0(&mut self, value: Int44) -> i32 {
        let mac = self.set_mac0(value);
        self.set_ir0(mac as i32);
        mac
    }

    pub fn set_mac_ir1(&mut self, value: Int44, shift: bool, no_neg: bool) -> i32 {
        let mac = self.inner_set_mac::<1>(value, shift);
        self.set_ir1(mac, no_neg);
        mac
    }

    pub fn set_mac_ir2(&mut self, value: Int44, shift: bool, no_neg: bool) -> i32 {
        let mac = self.inner_set_mac::<2>(value, shift);
        self.set_ir2(mac, no_neg);
        mac
    }

    pub fn set_mac_ir3(&mut self, value: Int44, shift: bool, no_neg: bool) -> i32 {
        let mac = self.inner_set_mac::<3>(value, shift);
        self.set_ir3(mac, no_neg);
        mac
    }

    pub fn set_flag(&mut self, flag: Flag, value: bool) {
        let reg = self.read(Reg::FLAG);
        self.write(Reg::FLAG, reg.with_bit(flag as u8, value));
    }

    pub fn merge_flag(&mut self, flag: Flag, value: bool) {
        let reg = self.read(Reg::FLAG);
        self.write(Reg::FLAG, reg | reg.with_bit(flag as u8, value));
    }

    pub fn push_xy(&mut self, x: i32, y: i32) {
        self.0[Reg::SXY0 as usize] = self.0[Reg::SXY1 as usize];
        self.0[Reg::SXY1 as usize] = self.0[Reg::SXY2 as usize];

        let s2: &mut [i16; 2] = transmute_mut!(&mut self.0[Reg::SXY2 as usize]);
        let (x, clamped_x) = clamp(x, -0x400, 0x3FF);
        let (y, clamped_y) = clamp(y, -0x400, 0x3FF);
        s2[0] = x as i16;
        s2[1] = y as i16;
        self.merge_flag(Flag::ClampedSX2, clamped_x);
        self.merge_flag(Flag::ClampedSY2, clamped_y);

        self.0[Reg::SXYP as usize] = self.0[Reg::SXY2 as usize];
    }

    pub fn push_z(&mut self, value: Int44) {
        self.0[Reg::SZ0 as usize] = self.0[Reg::SZ1 as usize];
        self.0[Reg::SZ1 as usize] = self.0[Reg::SZ2 as usize];
        self.0[Reg::SZ2 as usize] = self.0[Reg::SZ3 as usize];

        let value = value.value() >> 12;
        let (value, saturated) = if value < 0 {
            (0, true)
        } else if value > 0xffff {
            (0xffff, true)
        } else {
            (value, false)
        };

        self.0[Reg::SZ3 as usize] = value as u32;
        self.merge_flag(Flag::ClampedZ, saturated);
    }

    pub fn push_color(&mut self, r: i32, g: i32, b: i32) {
        self.0[Reg::RGB0 as usize] = self.0[Reg::RGB1 as usize];
        self.0[Reg::RGB1 as usize] = self.0[Reg::RGB2 as usize];

        let (r, clamped_r) = clamp(r, 0x00, 0xFF);
        let (g, clamped_g) = clamp(g, 0x00, 0xFF);
        let (b, clamped_b) = clamp(b, 0x00, 0xFF);
        let c = self.0[Reg::RGBC as usize].to_le_bytes()[3];

        self.write(
            Reg::RGB2,
            u32::from_le_bytes([r as u8, g as u8, b as u8, c]),
        );

        self.set_flag(Flag::ClampedR, clamped_r);
        self.set_flag(Flag::ClampedG, clamped_g);
        self.set_flag(Flag::ClampedB, clamped_b);
    }
}

#[derive(Default)]
pub struct Gte {
    pub regs: Registers,
}
