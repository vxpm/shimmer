//! Core crate of the shimmer PSX emulator. This crate defines core PSX structures in an
//! implementation independent way. The emulator implementation itself lives in the `shimmer` crate.

#![feature(inline_const_pat)]
#![feature(unbounded_shifts)]
#![feature(debug_closure_helpers)]
#![feature(let_chains)]

pub mod cdrom;
pub mod cpu;
pub mod dma;
pub mod exe;
pub mod gpu;
pub mod interrupts;
pub mod kernel;
pub mod mem;
pub mod sio0;
pub mod timers;

mod util;

pub use binrw;
