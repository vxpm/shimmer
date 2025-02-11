//! Core crate of the shimmer PSX emulator. This crate is intended to contain the actual emulator
//! functionality, but no "frontend" code such as a GUI or a CLI. It also does not perform any sort
//! of rendering: it only _provides_ the information necessary for a renderer to do it's job.

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
