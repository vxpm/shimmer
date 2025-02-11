#![feature(inline_const_pat)]
#![feature(unbounded_shifts)]
#![feature(debug_closure_helpers)]
#![feature(let_chains)]

pub mod cdrom;
pub mod cpu;
pub mod dma;
pub mod gpu;
pub mod scheduler;
pub mod sio0;
mod util;

use easyerr::{Error, ResultExt};
use scheduler::{Event, Scheduler};
use shimmer_core::cpu::cop0;
use std::path::PathBuf;
use tinylog::Logger;

/// All the loggers of the [`PSX`].
pub struct Loggers {
    pub root: Logger,
    pub bus: Logger,
    pub dma: Logger,
    pub cpu: Logger,
    pub kernel: Logger,
    pub gpu: Logger,
    pub cdrom: Logger,
    pub sio: Logger,
}

impl Loggers {
    pub fn new(logger: Logger) -> Self {
        Self {
            bus: logger.child("bus", tinylog::Level::Trace),
            dma: logger.child("dma", tinylog::Level::Trace),
            cpu: logger.child("cpu", tinylog::Level::Trace),
            kernel: logger.child("kernel", tinylog::Level::Trace),
            gpu: logger.child("gpu", tinylog::Level::Trace),
            cdrom: logger.child("cdrom", tinylog::Level::Trace),
            sio: logger.child("sio", tinylog::Level::Trace),
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

    pub memory: mem::Memory,
    pub timers: timers::Timers,
    pub dma: dma::Controller,
    pub cpu: cpu::Cpu,
    pub cop0: cop0::Cop0,
    pub interrupts: interrupts::Controller,
    pub gpu: gpu::Gpu,
    pub cdrom: cdrom::Controller,
    pub sio0: sio0::Controller,
}

/// Emulator configuration.
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

    /// The GPU command interpreter.
    gpu_interpreter: gpu::Interpreter,
    /// The DMA executor.
    dma_executor: dma::Executor,
    /// The CDROM command interpreter.
    cdrom_interpreter: cdrom::Interpreter,
    /// The SIO0 interpreter.
    sio0_interpreter: sio0::Interpreter,
}

impl Emulator {
    /// Creates a new [`Emulator`].
    pub fn new(
        config: Config,
        renderer: impl gpu::interface::Renderer + 'static,
    ) -> Result<Self, EmulatorError> {
        let gpu_interpreter = gpu::Interpreter::new(renderer);
        let loggers = Loggers::new(config.logger);

        let rom = config
            .rom_path
            .map(|path| std::fs::File::open(path).context(EmulatorCtx::RomOpen))
            .transpose()?;

        Ok(Self {
            psx: PSX {
                scheduler: Scheduler::new(),

                memory: mem::Memory::with_bios(config.bios).expect("BIOS should fit"),
                timers: timers::Timers::default(),
                dma: dma::Controller::default(),
                cpu: cpu::Cpu::default(),
                cop0: cop0::Cop0::default(),
                interrupts: interrupts::Controller::default(),
                gpu: gpu::Gpu::default(),
                cdrom: cdrom::Controller::new(rom, loggers.cdrom.clone()),
                sio0: sio0::Controller::default(),

                loggers,
            },
            dma_executor: dma::Executor::default(),
            gpu_interpreter,
            cdrom_interpreter: cdrom::Interpreter::default(),
            sio0_interpreter: sio0::Interpreter::default(),
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

    pub fn cycle_for(&mut self, cycles: u64) {
        let mut remaining = cycles;
        loop {
            let until_next = self.psx.scheduler.until_next().unwrap();
            if until_next <= remaining {
                self.psx.scheduler.advance(until_next);
                remaining -= until_next;
            } else {
                self.psx.scheduler.advance(remaining);
                return;
            }

            while let Some(e) = self.psx.scheduler.pop() {
                match e {
                    Event::Cpu => {
                        // TODO: make CPU like gpu interpreter, dma executor, etc

                        // stall cpu while DMA is ongoing
                        if self.dma_executor.ongoing() {
                            cold_path();
                            self.psx.scheduler.schedule(Event::Cpu, 16);
                            continue;
                        }

                        let mut interpreter = cpu::Interpreter::new(&mut self.psx);
                        let cycles = interpreter.exec_next();

                        self.psx.scheduler.schedule(Event::Cpu, cycles);
                    }
                    Event::VBlank => {
                        self.gpu_interpreter.vblank(&mut self.psx);
                    }
                    Event::Timer1 => {
                        let cycles = self.psx.timers.timer1.tick();
                        self.psx.scheduler.schedule(Event::Timer1, cycles);
                    }
                    Event::Timer2 => {
                        let cycles = self.psx.timers.timer2.tick();
                        self.psx.scheduler.schedule(Event::Timer2, cycles);
                    }
                    Event::Gpu => {
                        self.gpu_interpreter.exec_queued(&mut self.psx);
                    }
                    Event::DmaUpdate => {
                        self.dma_executor.update(&mut self.psx);
                    }
                    Event::DmaAdvance => {
                        self.dma_executor.advance(&mut self.psx);
                    }
                    Event::Cdrom(event) => {
                        self.cdrom_interpreter.update(&mut self.psx, event);
                    }
                    Event::Sio(event) => {
                        self.sio0_interpreter.update(&mut self.psx, event);
                    }
                }
            }
        }
    }
}
