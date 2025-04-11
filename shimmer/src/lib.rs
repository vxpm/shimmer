//! Main crate of the shimmer PSX emulator. This crate is reponsible for implementing the emulation
//! of the PSX, providing all of the functionality but no "frontend" code such as a GUI or a CLI.
//!
//! Consequently, it does not perform any sort of rendering: the GPU exposes a rendering interface
//! for renderer implementations.

#![feature(inline_const_pat)]
#![feature(debug_closure_helpers)]
#![feature(let_chains)]
#![feature(cold_path)]
#![feature(int_roundings)]

mod bus;
pub mod cdrom;
pub mod cpu;
pub mod dma;
pub mod gpu;
pub mod scheduler;
pub mod sio0;
pub mod timers;

use cdrom::Rom;
use easyerr::{Error, ResultExt};
use scheduler::{Event, Scheduler};
use shimmer_core::{
    cdrom::Cdrom,
    cpu::{Cpu, cop0::Cop0},
    dma::Controller as DmaController,
    gpu::Gpu,
    gte::Gte,
    interrupts::Controller as InterruptController,
    mem::Memory,
    sio0::Sio0,
    timers::Timers,
};
use sio0::Joypad;
use std::{hint::cold_path, path::PathBuf};
use tinylog::Logger;

pub use shimmer_core as core;

/// All the loggers of the [`PSX`].
pub struct Loggers {
    pub root: Logger,
    pub bus: Logger,
    pub dma: Logger,
    pub cpu: Logger,
    pub gte: Logger,
    pub kernel: Logger,
    pub gpu: Logger,
    pub cdrom: Logger,
    pub sio: Logger,
    pub timers: Logger,
}

impl Loggers {
    pub fn new(logger: Logger) -> Self {
        Self {
            bus: logger.child("bus", tinylog::Level::Trace),
            dma: logger.child("dma", tinylog::Level::Trace),
            cpu: logger.child("cpu", tinylog::Level::Trace),
            gte: logger.child("gte", tinylog::Level::Trace),
            kernel: logger.child("kernel", tinylog::Level::Trace),
            gpu: logger.child("gpu", tinylog::Level::Trace),
            cdrom: logger.child("cdrom", tinylog::Level::Trace),
            sio: logger.child("sio", tinylog::Level::Trace),
            timers: logger.child("timers", tinylog::Level::Trace),
            root: logger,
        }
    }
}

/// The state of the PSX. [`Emulator`] and it's systems operate on this struct.
pub struct PSX {
    /// The event scheduler.
    pub scheduler: Scheduler,
    /// The loggers of this [`PSX`].
    pub loggers: Loggers,

    pub memory: Memory,
    pub timers: Timers,
    pub dma: DmaController,
    pub cpu: Cpu,
    pub cop0: Cop0,
    pub gte: Gte,
    pub interrupts: InterruptController,
    pub gpu: Gpu,
    pub cdrom: Cdrom,
    pub sio0: Sio0,
}

/// Emulator configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// The BIOS ROM data.
    pub bios: Vec<u8>,
    /// The path to the ROM to run.
    pub rom_path: Option<PathBuf>,
    /// The root logger to use.
    pub logger: Logger,
}

#[derive(Debug, Error)]
pub enum EmulatorError {
    #[error("couldn't open ROM file")]
    RomOpen { source: std::io::Error },
}

/// The shimmer emulator.
pub struct Emulator {
    /// The state of the system.
    psx: PSX,

    cpu: cpu::Interpreter,
    gpu: gpu::Gpu,
    dma: dma::Dma,
    cdrom: cdrom::Cdrom,
    sio0: sio0::Sio0,
    timers: timers::Timers,
}

impl Emulator {
    /// Creates a new [`Emulator`].
    pub fn new(
        config: Config,
        renderer: impl gpu::interface::Renderer + 'static,
    ) -> Result<Self, EmulatorError> {
        let gpu = gpu::Gpu::new(renderer);
        let loggers = Loggers::new(config.logger);

        let rom = config
            .rom_path
            .map(|path| std::fs::File::open(path).context(EmulatorCtx::RomOpen))
            .transpose()?;

        Ok(Self {
            cpu: cpu::Interpreter::default(),
            gpu,
            dma: dma::Dma::default(),
            cdrom: cdrom::Cdrom::new(rom.map(|r| {
                let boxed: Box<dyn Rom> = Box::new(r);
                boxed
            })),
            sio0: sio0::Sio0::default(),
            timers: timers::Timers::new(loggers.timers.clone()),

            psx: PSX {
                scheduler: Scheduler::new(),

                memory: Memory::with_bios(config.bios).expect("BIOS should fit"),
                timers: Timers::default(),
                dma: DmaController::default(),
                cpu: Cpu::default(),
                cop0: Cop0::default(),
                gte: Gte::default(),
                interrupts: InterruptController::default(),
                gpu: Gpu::default(),
                cdrom: Cdrom::new(loggers.cdrom.clone()),
                sio0: Sio0::default(),

                loggers,
            },
        })
    }

    /// Returns a reference to the state of the system.
    #[inline(always)]
    pub fn psx(&mut self) -> &PSX {
        &self.psx
    }

    /// Returns a mutable reference to the state of the system.
    #[inline(always)]
    pub fn psx_mut(&mut self) -> &mut PSX {
        &mut self.psx
    }

    pub fn joypad_mut(&mut self) -> &mut Joypad {
        self.sio0.joypad_mut()
    }

    pub fn cdrom_mut(&mut self) -> &mut cdrom::Cdrom {
        &mut self.cdrom
    }

    pub fn cpu(&self) -> &cpu::Interpreter {
        &self.cpu
    }

    pub fn process_event(&mut self, event: Event) {
        match event {
            Event::VBlank => {
                self.gpu.vblank(&mut self.psx);
            }
            Event::Timer(event) => {
                self.timers.update(&mut self.psx, event);
            }
            Event::Gpu => {
                self.gpu.exec_queued(&mut self.psx);
            }
            Event::DmaUpdate => {
                self.dma.update(&mut self.psx);
            }
            Event::DmaAdvance => {
                self.dma.advance(&mut self.psx);
            }
            Event::Cdrom(event) => {
                self.cdrom.update(&mut self.psx, event);
            }
            Event::Sio(event) => {
                self.sio0.update(&mut self.psx, event);
            }
        }
    }

    fn exec_until_next_event(&mut self, limit: u64) -> u64 {
        let mut cycles = 0;
        let mut remaining = self.psx.scheduler.until_next().unwrap_or(limit).min(limit);
        let mut time_at_event = self.psx.scheduler.elapsed() + remaining;

        while remaining > 0 {
            if self.psx.scheduler.last_scheduled_time() < time_at_event {
                remaining = self.psx.scheduler.until_next().unwrap().min(limit);
                time_at_event = self.psx.scheduler.last_scheduled_time();
                continue;
            }

            // stall CPU while DMA is ongoing
            let elapsed = if self.dma.ongoing() {
                cold_path();
                1
            } else {
                self.cpu.exec_next(&mut self.psx)
            };

            // HACK: trades some precision for ease of implementation, shouldn't matter much. most
            // noticeable effect is slightly higher emulation speed, but it's still pretty minor
            cycles += elapsed.min(remaining);
            remaining -= elapsed.min(remaining);
        }

        cycles
    }

    pub fn cycle_for(&mut self, cycles: u64) {
        let mut remaining = cycles;
        while remaining > 0 {
            let executed = self.exec_until_next_event(remaining);
            self.psx.scheduler.advance(executed);
            remaining -= executed;

            while let Some(event) = self.psx.scheduler.pop() {
                self.process_event(event);
            }
        }
    }
}
