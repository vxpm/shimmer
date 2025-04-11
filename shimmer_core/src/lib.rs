//! Core crate of the shimmer PSX emulator. This crate defines core PSX structures in an
//! implementation independent way. The emulator implementation itself lives in the `shimmer` crate.

#![feature(inline_const_pat)]
#![feature(debug_closure_helpers)]
#![feature(let_chains)]

pub mod cdrom;
pub mod cpu;
pub mod dma;
pub mod exe;
pub mod gpu;
pub mod gte;
pub mod interrupts;
pub mod kernel;
pub mod mem;
pub mod sio0;
pub mod timers;

mod util;

pub type Cycles = u64;
pub const CYCLES_SECOND: Cycles = cpu::FREQUENCY as u64;
pub const CYCLES_MILLIS: Cycles = CYCLES_SECOND / 1000;
pub const CYCLES_MICROS: Cycles = CYCLES_MILLIS / 1000;

pub use binrw;
