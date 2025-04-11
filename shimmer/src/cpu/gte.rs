use super::Interpreter;
use crate::PSX;
use shimmer_core::gte::{
    Flag, Int44, Reg,
    instr::{Instruction, MulMatrix, MulVector, OffVector, Opcode},
};
use std::ops::{Add, Mul};
use tinylog::{debug, error};
use zerocopy::transmute;

#[inline(always)]
fn i44(value: i64) -> Int44 {
    Int44::new(value)
}

// this one is dark magic - just accept it
fn newton_raphson_div(lhs: u32, rhs: u32) -> (u32, bool) {
    fn reciprocal(divisor: u16) -> u32 {
        #[rustfmt::skip]
        static LUT: &[u8] = &[
            0xFF, 0xFD, 0xFB, 0xF9, 0xF7, 0xF5, 0xF3, 0xF1, 0xEF, 0xEE, 0xEC, 0xEA, 0xE8, 0xE6, 0xE4, 0xE3,
            0xE1, 0xDF, 0xDD, 0xDC, 0xDA, 0xD8, 0xD6, 0xD5, 0xD3, 0xD1, 0xD0, 0xCE, 0xCD, 0xCB, 0xC9, 0xC8,
            0xC6, 0xC5, 0xC3, 0xC1, 0xC0, 0xBE, 0xBD, 0xBB, 0xBA, 0xB8, 0xB7, 0xB5, 0xB4, 0xB2, 0xB1, 0xB0,
            0xAE, 0xAD, 0xAB, 0xAA, 0xA9, 0xA7, 0xA6, 0xA4, 0xA3, 0xA2, 0xA0, 0x9F, 0x9E, 0x9C, 0x9B, 0x9A,
            0x99, 0x97, 0x96, 0x95, 0x94, 0x92, 0x91, 0x90, 0x8F, 0x8D, 0x8C, 0x8B, 0x8A, 0x89, 0x87, 0x86,
            0x85, 0x84, 0x83, 0x82, 0x81, 0x7F, 0x7E, 0x7D, 0x7C, 0x7B, 0x7A, 0x79, 0x78, 0x77, 0x75, 0x74,
            0x73, 0x72, 0x71, 0x70, 0x6F, 0x6E, 0x6D, 0x6C, 0x6B, 0x6A, 0x69, 0x68, 0x67, 0x66, 0x65, 0x64,
            0x63, 0x62, 0x61, 0x60, 0x5F, 0x5E, 0x5D, 0x5D, 0x5C, 0x5B, 0x5A, 0x59, 0x58, 0x57, 0x56, 0x55,
            0x54, 0x53, 0x53, 0x52, 0x51, 0x50, 0x4F, 0x4E, 0x4D, 0x4D, 0x4C, 0x4B, 0x4A, 0x49, 0x48, 0x48,
            0x47, 0x46, 0x45, 0x44, 0x43, 0x43, 0x42, 0x41, 0x40, 0x3F, 0x3F, 0x3E, 0x3D, 0x3C, 0x3C, 0x3B,
            0x3A, 0x39, 0x39, 0x38, 0x37, 0x36, 0x36, 0x35, 0x34, 0x33, 0x33, 0x32, 0x31, 0x31, 0x30, 0x2F,
            0x2E, 0x2E, 0x2D, 0x2C, 0x2C, 0x2B, 0x2A, 0x2A, 0x29, 0x28, 0x28, 0x27, 0x26, 0x26, 0x25, 0x24,
            0x24, 0x23, 0x22, 0x22, 0x21, 0x20, 0x20, 0x1F, 0x1E, 0x1E, 0x1D, 0x1D, 0x1C, 0x1B, 0x1B, 0x1A,
            0x19, 0x19, 0x18, 0x18, 0x17, 0x16, 0x16, 0x15, 0x15, 0x14, 0x14, 0x13, 0x12, 0x12, 0x11, 0x11,
            0x10, 0x0F, 0x0F, 0x0E, 0x0E, 0x0D, 0x0D, 0x0C, 0x0C, 0x0B, 0x0A, 0x0A, 0x09, 0x09, 0x08, 0x08,
            0x07, 0x07, 0x06, 0x06, 0x05, 0x05, 0x04, 0x04, 0x03, 0x03, 0x02, 0x02, 0x01, 0x01, 0x00, 0x00,
            0x00
        ];

        let index = ((divisor & 0x7FFF) + 0x40) >> 7;
        let x = 0x101 + LUT[index as usize] as i32;
        let iter1 = (((divisor as i32) * -x) + 0x80) >> 8;
        let iter2 = ((x * (0x20000 + iter1)) + 0x80) >> 8;

        iter2 as u32
    }

    if !(2 * rhs > lhs) {
        return (0x1FFFF, true);
    }

    let shift = (rhs as u16).leading_zeros();
    let (lhs, rhs) = (lhs << shift, rhs << shift);
    let reciprocal = reciprocal((rhs | 0x8000) as u16);
    let result = (((lhs as u64) * (reciprocal as u64) + 0x8000) >> 16) as u32;

    (result.min(0x1FFFF), false)
}

#[derive(Debug, Clone, Copy)]
struct Vector {
    x: Int44,
    y: Int44,
    z: Int44,
}

impl Vector {
    pub fn new(x: Int44, y: Int44, z: Int44) -> Self {
        Self { x, y, z }
    }

    pub fn zero() -> Self {
        Self {
            x: i44(0),
            y: i44(0),
            z: i44(0),
        }
    }
}

impl Add for Vector {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        Vector {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl Mul<Vector> for [[Int44; 3]; 3] {
    type Output = Vector;

    #[inline(always)]
    fn mul(self, v: Vector) -> Self::Output {
        Vector {
            x: self[0][0] * v.x + self[0][1] * v.y + self[0][2] * v.z,
            y: self[1][0] * v.x + self[1][1] * v.y + self[1][2] * v.z,
            z: self[2][0] * v.x + self[2][1] * v.y + self[2][2] * v.z,
        }
    }
}

#[inline(always)]
fn translation_vector(psx: &PSX) -> Vector {
    Vector {
        x: i44((psx.gte.regs.read(Reg::TRX) as i64) << 12),
        y: i44((psx.gte.regs.read(Reg::TRY) as i64) << 12),
        z: i44((psx.gte.regs.read(Reg::TRZ) as i64) << 12),
    }
}

#[inline(always)]
fn background_color_vector(psx: &PSX) -> Vector {
    Vector {
        x: i44((psx.gte.regs.read(Reg::BCR) as i64) << 12),
        y: i44((psx.gte.regs.read(Reg::BCG) as i64) << 12),
        z: i44((psx.gte.regs.read(Reg::BCB) as i64) << 12),
    }
}

#[inline(always)]
fn far_color_vector(psx: &PSX) -> Vector {
    Vector {
        x: i44((psx.gte.regs.read(Reg::FCR) as i64) << 12),
        y: i44((psx.gte.regs.read(Reg::FCG) as i64) << 12),
        z: i44((psx.gte.regs.read(Reg::FCB) as i64) << 12),
    }
}

#[inline(always)]
fn rotation_matrix(psx: &PSX) -> [[Int44; 3]; 3] {
    let rt_11_12: [i16; 2] = transmute!(psx.gte.regs.read(Reg::RT_11_12));
    let rt_13_21: [i16; 2] = transmute!(psx.gte.regs.read(Reg::RT_13_21));
    let rt_22_23: [i16; 2] = transmute!(psx.gte.regs.read(Reg::RT_22_23));
    let rt_31_32: [i16; 2] = transmute!(psx.gte.regs.read(Reg::RT_31_32));
    let rt_33_ss: [i16; 2] = transmute!(psx.gte.regs.read(Reg::RT_33_SS));

    [
        [rt_11_12[0], rt_11_12[1], rt_13_21[0]],
        [rt_13_21[1], rt_22_23[0], rt_22_23[1]],
        [rt_31_32[0], rt_31_32[1], rt_33_ss[0]],
    ]
    .map(|x| x.map(|x| i44(x as i64)))
}

#[inline(always)]
fn light_matrix(psx: &PSX) -> [[Int44; 3]; 3] {
    let l_11_12: [i16; 2] = transmute!(psx.gte.regs.read(Reg::L_11_12));
    let l_13_21: [i16; 2] = transmute!(psx.gte.regs.read(Reg::L_13_21));
    let l_22_23: [i16; 2] = transmute!(psx.gte.regs.read(Reg::L_22_23));
    let l_31_32: [i16; 2] = transmute!(psx.gte.regs.read(Reg::L_31_32));
    let l_33_ss: [i16; 2] = transmute!(psx.gte.regs.read(Reg::L_33_SS));

    [
        [l_11_12[0], l_11_12[1], l_13_21[0]],
        [l_13_21[1], l_22_23[0], l_22_23[1]],
        [l_31_32[0], l_31_32[1], l_33_ss[0]],
    ]
    .map(|x| x.map(|x| i44(x as i64)))
}

#[inline(always)]
fn color_matrix(psx: &PSX) -> [[Int44; 3]; 3] {
    let c_11_12: [i16; 2] = transmute!(psx.gte.regs.read(Reg::L_R1_R2));
    let c_13_21: [i16; 2] = transmute!(psx.gte.regs.read(Reg::L_R3_G1));
    let c_22_23: [i16; 2] = transmute!(psx.gte.regs.read(Reg::L_G2_G3));
    let c_31_32: [i16; 2] = transmute!(psx.gte.regs.read(Reg::L_B1_B2));
    let c_33_ss: [i16; 2] = transmute!(psx.gte.regs.read(Reg::L_B3_SS));

    [
        [c_11_12[0], c_11_12[1], c_13_21[0]],
        [c_13_21[1], c_22_23[0], c_22_23[1]],
        [c_31_32[0], c_31_32[1], c_33_ss[0]],
    ]
    .map(|x| x.map(|x| i44(x as i64)))
}

#[inline(always)]
fn vector0(psx: &PSX) -> Vector {
    let vxy: [i16; 2] = transmute!(psx.gte.regs.read(Reg::VXY0));
    let vz: [i16; 2] = transmute!(psx.gte.regs.read(Reg::VZ0));

    Vector {
        x: i44(vxy[0] as i64),
        y: i44(vxy[1] as i64),
        z: i44(vz[0] as i64),
    }
}

#[inline(always)]
fn vector1(psx: &PSX) -> Vector {
    let vxy: [i16; 2] = transmute!(psx.gte.regs.read(Reg::VXY1));
    let vz: [i16; 2] = transmute!(psx.gte.regs.read(Reg::VZ1));

    Vector {
        x: i44(vxy[0] as i64),
        y: i44(vxy[1] as i64),
        z: i44(vz[0] as i64),
    }
}

#[inline(always)]
fn vector2(psx: &PSX) -> Vector {
    let vxy: [i16; 2] = transmute!(psx.gte.regs.read(Reg::VXY2));
    let vz: [i16; 2] = transmute!(psx.gte.regs.read(Reg::VZ2));

    Vector {
        x: i44(vxy[0] as i64),
        y: i44(vxy[1] as i64),
        z: i44(vz[0] as i64),
    }
}

fn rtps<const MAC0: bool>(psx: &mut PSX, vector: Vector, instr: Instruction) {
    let rotation = rotation_matrix(psx);
    let translation = translation_vector(psx);
    let r = translation + rotation * vector;

    psx.gte.regs.set_mac_ir1(r.x, instr.shift(), instr.no_neg());
    psx.gte.regs.set_mac_ir2(r.y, instr.shift(), instr.no_neg());
    psx.gte.regs.set_mac_ir3(r.z, instr.shift(), instr.no_neg());
    psx.gte.regs.push_z(r.z);

    // NOTE: IR3 has a hardware bug where it always shifts the fraction and considers the full
    // clamping range for the flag
    let clamped_ir3 = (r.z.value() >> 12) < -0x8000 || (r.z.value() >> 12) > 0x7FFF;
    psx.gte.regs.set_flag(Flag::ClampedIR3, clamped_ir3);

    // NOTE: undo GTE sign-extending bug
    let h = psx.gte.regs.read(Reg::H) as u16 as u32;
    let sz3 = psx.gte.regs.read(Reg::SZ3);

    let (h_by_sz3, overflow) = newton_raphson_div(h, sz3);
    psx.gte.regs.set_flag(Flag::DivideOverflow, overflow);

    let h_by_sz3 = i44(h_by_sz3 as i64);
    let ir1 = i44(psx.gte.regs.read(Reg::IR1) as i32 as i64);
    let ir2 = i44(psx.gte.regs.read(Reg::IR2) as i32 as i64);
    let ofx = i44(psx.gte.regs.read(Reg::OFX) as i32 as i64);
    let ofy = i44(psx.gte.regs.read(Reg::OFY) as i32 as i64);
    let x = h_by_sz3 * ir1 + ofx;
    let y = h_by_sz3 * ir2 + ofy;
    psx.gte.regs.set_mac0(x);
    psx.gte.regs.set_mac0(y);
    psx.gte
        .regs
        .push_xy((x.value() >> 16) as i32, (y.value() >> 16) as i32);

    if MAC0 {
        let dqa = i44(psx.gte.regs.read(Reg::DQA) as i32 as i64);
        let dqb = i44(psx.gte.regs.read(Reg::DQB) as i32 as i64);

        let mac0 = h_by_sz3 * dqa + dqb;
        psx.gte.regs.set_mac0(mac0);
        psx.gte.regs.set_ir0((mac0.value() >> 12) as i32);
    }
}

fn rtpt(psx: &mut PSX, instr: Instruction) {
    rtps::<false>(psx, vector0(psx), instr);
    rtps::<false>(psx, vector1(psx), instr);
    rtps::<true>(psx, vector2(psx), instr);
}

fn nclip(psx: &mut PSX, _: Instruction) {
    let s0: [i16; 2] = transmute!(psx.gte.regs.read(Reg::SXY0));
    let s1: [i16; 2] = transmute!(psx.gte.regs.read(Reg::SXY1));
    let s2: [i16; 2] = transmute!(psx.gte.regs.read(Reg::SXY2));

    let (sx0, sy0) = (s0[0] as i64, s0[1] as i64);
    let (sx1, sy1) = (s1[0] as i64, s1[1] as i64);
    let (sx2, sy2) = (s2[0] as i64, s2[1] as i64);

    let result = sx0 * sy1 + sx1 * sy2 + sx2 * sy0 - sx0 * sy2 - sx1 * sy0 - sx2 * sy1;
    psx.gte.regs.set_mac0(i44(result));
}

fn avsz3(psx: &mut PSX, _: Instruction) {
    let zsf3 = i44(psx.gte.regs.read(Reg::ZSF3) as i32 as i64);
    let sz1 = i44(psx.gte.regs.read(Reg::SZ1) as i32 as i64);
    let sz2 = i44(psx.gte.regs.read(Reg::SZ2) as i32 as i64);
    let sz3 = i44(psx.gte.regs.read(Reg::SZ3) as i32 as i64);
    let avg = zsf3 * (sz1 + sz2 + sz3);
    psx.gte.regs.set_mac0(avg);

    let otz = (avg.value() >> 12) as i32;
    let (otz, clamped) = if otz > 0xFFFF {
        (0xFFFF, true)
    } else if otz < 0 {
        (0, true)
    } else {
        (otz, false)
    };

    psx.gte.regs.merge_flag(Flag::ClampedZ, clamped);
    psx.gte.regs.write(Reg::OTZ, otz as u32);
}

fn avsz4(psx: &mut PSX, _: Instruction) {
    let zsf4 = i44(psx.gte.regs.read(Reg::ZSF4) as i32 as i64);
    let sz0 = i44(psx.gte.regs.read(Reg::SZ0) as i32 as i64);
    let sz1 = i44(psx.gte.regs.read(Reg::SZ1) as i32 as i64);
    let sz2 = i44(psx.gte.regs.read(Reg::SZ2) as i32 as i64);
    let sz3 = i44(psx.gte.regs.read(Reg::SZ3) as i32 as i64);
    let avg = zsf4 * (sz0 + sz1 + sz2 + sz3);
    psx.gte.regs.set_mac0(avg);

    let otz = (avg.value() >> 12) as i32;
    let (otz, clamped) = if otz > 0xFFFF {
        (0xFFFF, true)
    } else if otz < 0 {
        (0, true)
    } else {
        (otz, false)
    };

    psx.gte.regs.merge_flag(Flag::ClampedZ, clamped);
    psx.gte.regs.write(Reg::OTZ, otz as u32);
}

fn cross(psx: &mut PSX, instr: Instruction) {
    let ir1 = i44(psx.gte.regs.read(Reg::IR1) as i32 as i64);
    let ir2 = i44(psx.gte.regs.read(Reg::IR2) as i32 as i64);
    let ir3 = i44(psx.gte.regs.read(Reg::IR3) as i32 as i64);
    let d1 = i44(psx.gte.regs.read(Reg::RT_11_12) as i16 as i64);
    let d2 = i44(psx.gte.regs.read(Reg::RT_22_23) as i16 as i64);
    let d3 = i44(psx.gte.regs.read(Reg::RT_33_SS) as i16 as i64);

    psx.gte
        .regs
        .set_mac_ir1(ir3 * d2 - ir2 * d3, instr.shift(), instr.no_neg());
    psx.gte
        .regs
        .set_mac_ir2(ir1 * d3 - ir3 * d1, instr.shift(), instr.no_neg());
    psx.gte
        .regs
        .set_mac_ir3(ir2 * d1 - ir1 * d2, instr.shift(), instr.no_neg());
}

fn interpolate_color(
    psx: &mut PSX,
    mac1: Int44,
    mac2: Int44,
    mac3: Int44,
    shift: bool,
    no_neg: bool,
) {
    psx.gte.regs.set_mac1(mac1, false);
    psx.gte.regs.set_mac2(mac2, false);
    psx.gte.regs.set_mac3(mac3, false);

    let fcr = i44((psx.gte.regs.read(Reg::FCR) as i32 as i64) << 12);
    let fcg = i44((psx.gte.regs.read(Reg::FCG) as i32 as i64) << 12);
    let fcb = i44((psx.gte.regs.read(Reg::FCB) as i32 as i64) << 12);
    psx.gte.regs.set_mac_ir1(fcr - mac1, shift, false);
    psx.gte.regs.set_mac_ir2(fcg - mac2, shift, false);
    psx.gte.regs.set_mac_ir3(fcb - mac3, shift, false);

    let ir0 = i44(psx.gte.regs.read(Reg::IR0) as i32 as i64);
    let ir1 = i44(psx.gte.regs.read(Reg::IR1) as i32 as i64);
    let ir2 = i44(psx.gte.regs.read(Reg::IR2) as i32 as i64);
    let ir3 = i44(psx.gte.regs.read(Reg::IR3) as i32 as i64);
    psx.gte.regs.set_mac_ir1(ir0 * ir1 + mac1, shift, no_neg);
    psx.gte.regs.set_mac_ir2(ir0 * ir2 + mac2, shift, no_neg);
    psx.gte.regs.set_mac_ir3(ir0 * ir3 + mac3, shift, no_neg);
}

fn dpcs(psx: &mut PSX, instr: Instruction) {
    let [r, g, b, _]: [u8; 4] = transmute!(psx.gte.regs.read(Reg::RGBC));
    let r = i44((r as u64 as i64) << 16);
    let g = i44((g as u64 as i64) << 16);
    let b = i44((b as u64 as i64) << 16);

    interpolate_color(psx, r, g, b, instr.shift(), instr.no_neg());

    let mac1 = psx.gte.regs.read(Reg::MAC1) as i32;
    let mac2 = psx.gte.regs.read(Reg::MAC2) as i32;
    let mac3 = psx.gte.regs.read(Reg::MAC3) as i32;
    psx.gte.regs.push_color(mac1 >> 4, mac2 >> 4, mac3 >> 4);
}

fn intpl(psx: &mut PSX, instr: Instruction) {
    let ir1 = i44((psx.gte.regs.read(Reg::IR1) as i64) << 12);
    let ir2 = i44((psx.gte.regs.read(Reg::IR2) as i64) << 12);
    let ir3 = i44((psx.gte.regs.read(Reg::IR3) as i64) << 12);

    interpolate_color(psx, ir1, ir2, ir3, instr.shift(), instr.no_neg());

    let mac1 = psx.gte.regs.read(Reg::MAC1) as i32;
    let mac2 = psx.gte.regs.read(Reg::MAC2) as i32;
    let mac3 = psx.gte.regs.read(Reg::MAC3) as i32;
    psx.gte.regs.push_color(mac1 >> 4, mac2 >> 4, mac3 >> 4);
}

fn mvmva(psx: &mut PSX, instr: Instruction) {
    let matrix = match instr.multiply_matrix() {
        MulMatrix::Rotation => rotation_matrix(psx),
        MulMatrix::Light => light_matrix(psx),
        MulMatrix::Color => color_matrix(psx),
        MulMatrix::Reserved => {
            let r = ((psx.gte.regs.read(Reg::RGBC) as u8 as u16) << 4) as i64;
            let ir0 = i44(psx.gte.regs.read(Reg::IR0) as i16 as i64);
            let rot_matrix = rotation_matrix(psx);
            [
                [i44(-r), i44(r), ir0],
                [rot_matrix[0][2]; 3],
                [rot_matrix[1][1]; 3],
            ]
        }
    };

    let vector = match instr.multiply_vector() {
        MulVector::Vector0 => vector0(psx),
        MulVector::Vector1 => vector1(psx),
        MulVector::Vector2 => vector2(psx),
        MulVector::IR => {
            let ir1 = i44(psx.gte.regs.read(Reg::IR1) as i32 as i64);
            let ir2 = i44(psx.gte.regs.read(Reg::IR2) as i32 as i64);
            let ir3 = i44(psx.gte.regs.read(Reg::IR3) as i32 as i64);

            Vector {
                x: ir1,
                y: ir2,
                z: ir3,
            }
        }
    };

    let offset = match instr.offset_vector() {
        OffVector::Translation => translation_vector(psx),
        OffVector::BackgroundColor => background_color_vector(psx),
        OffVector::FarColor => far_color_vector(psx),
        OffVector::None => Vector::zero(),
    };

    if instr.offset_vector() == OffVector::FarColor {
        let flag = Vector {
            x: offset.x + matrix[0][0] * vector.x,
            y: offset.y + matrix[1][0] * vector.x,
            z: offset.z + matrix[2][0] * vector.x,
        };

        psx.gte.regs.set_mac_ir1(flag.x, instr.shift(), false);
        psx.gte.regs.set_mac_ir2(flag.y, instr.shift(), false);
        psx.gte.regs.set_mac_ir3(flag.z, instr.shift(), false);

        let r = Vector {
            x: matrix[0][1] * vector.y + matrix[0][2] * vector.z,
            y: matrix[1][1] * vector.y + matrix[1][2] * vector.z,
            z: matrix[2][1] * vector.y + matrix[2][2] * vector.z,
        };

        psx.gte.regs.set_mac_ir1(r.x, instr.shift(), instr.no_neg());
        psx.gte.regs.set_mac_ir2(r.y, instr.shift(), instr.no_neg());
        psx.gte.regs.set_mac_ir3(r.z, instr.shift(), instr.no_neg());
    } else {
        let r = offset + matrix * vector;
        psx.gte.regs.set_mac_ir1(r.x, instr.shift(), instr.no_neg());
        psx.gte.regs.set_mac_ir2(r.y, instr.shift(), instr.no_neg());
        psx.gte.regs.set_mac_ir3(r.z, instr.shift(), instr.no_neg());
    }
}

fn ncds(psx: &mut PSX, instr: Instruction) {
    let light_matrix = light_matrix(psx);
    let color_matrix = color_matrix(psx);
    let background_color_vector = background_color_vector(psx);
    let v0 = vector0(psx);

    let v = light_matrix * v0;
    psx.gte.regs.set_mac_ir1(v.x, instr.shift(), instr.no_neg());
    psx.gte.regs.set_mac_ir2(v.y, instr.shift(), instr.no_neg());
    psx.gte.regs.set_mac_ir3(v.z, instr.shift(), instr.no_neg());

    let ir1 = psx.gte.regs.read(Reg::IR1) as i32 as i64;
    let ir2 = psx.gte.regs.read(Reg::IR2) as i32 as i64;
    let ir3 = psx.gte.regs.read(Reg::IR3) as i32 as i64;
    let ir = Vector::new(i44(ir1), i44(ir2), i44(ir3));
    let v = background_color_vector + color_matrix * ir;
    psx.gte.regs.set_mac_ir1(v.x, instr.shift(), instr.no_neg());
    psx.gte.regs.set_mac_ir2(v.y, instr.shift(), instr.no_neg());
    psx.gte.regs.set_mac_ir3(v.z, instr.shift(), instr.no_neg());

    let [r, g, b, _]: [u8; 4] = transmute!(psx.gte.regs.read(Reg::RGBC));
    let r = i44((r as i64) << 4);
    let g = i44((g as i64) << 4);
    let b = i44((b as i64) << 4);
    let ir1 = i44(psx.gte.regs.read(Reg::IR1) as i32 as i64);
    let ir2 = i44(psx.gte.regs.read(Reg::IR2) as i32 as i64);
    let ir3 = i44(psx.gte.regs.read(Reg::IR3) as i32 as i64);

    interpolate_color(
        psx,
        r * ir1,
        g * ir2,
        b * ir3,
        instr.shift(),
        instr.no_neg(),
    );

    let mac1 = psx.gte.regs.read(Reg::MAC1) as i32;
    let mac2 = psx.gte.regs.read(Reg::MAC2) as i32;
    let mac3 = psx.gte.regs.read(Reg::MAC3) as i32;
    psx.gte.regs.push_color(mac1 >> 4, mac2 >> 4, mac3 >> 4);
}

impl Interpreter {
    pub fn exec_gte(&mut self, psx: &mut PSX, instr: Instruction) {
        let Some(op) = instr.op() else {
            error!(
                psx.loggers.gte,
                "executing unknown: 0x{:02X}",
                instr.op_raw()
            );

            return;
        };

        debug!(psx.loggers.gte, "executing {op:?}");

        psx.gte.regs.write(Reg::FLAG, 0);
        match op {
            Opcode::RTPS => rtps::<true>(psx, vector0(psx), instr),
            Opcode::NCLIP => nclip(psx, instr),
            Opcode::NCDS => ncds(psx, instr),
            Opcode::AVSZ3 => avsz3(psx, instr),
            Opcode::RTPT => rtpt(psx, instr),
            Opcode::INTPL => intpl(psx, instr),
            Opcode::OP => cross(psx, instr),
            Opcode::DPCS => dpcs(psx, instr),
            Opcode::MVMVA => mvmva(psx, instr),
            Opcode::AVSZ4 => avsz4(psx, instr),
        }
    }
}
