//! Core crate of the shimmer PSX emulator. This crate is intended to contain the actual emulator
//! functionality, but no "frontend" code such as a GUI or a CLI. It also does not perform any sort
//! of rendering: it only _provides_ the information necessary for a renderer to do it's job.

#![feature(inline_const_pat)]
#![feature(unbounded_shifts)]
#![feature(debug_closure_helpers)]
#![feature(let_chains)]

pub mod cpu;
pub mod dma;
pub mod exe;
pub mod gpu;
pub mod interrupts;
pub mod kernel;
pub mod mem;
pub mod timers;

mod scheduler;
mod util;

use cpu::cop0;
use interrupts::Interrupt;
use scheduler::{Event, Scheduler};
use std::sync::mpsc::Receiver;
use tinylog::Logger;

pub use binrw;
use util::cold_path;

/// All the loggers of the [`PSX`].
pub struct Loggers {
    pub root: Logger,
    pub bus: Logger,
    pub dma: Logger,
    pub cpu: Logger,
    pub kernel: Logger,
    pub gpu: Logger,
}

impl Loggers {
    pub fn new(logger: Logger) -> Self {
        Self {
            bus: logger.child("bus", tinylog::Level::Trace),
            dma: logger.child("dma", tinylog::Level::Trace),
            cpu: logger.child("cpu", tinylog::Level::Trace),
            kernel: logger.child("kernel", tinylog::Level::Trace),
            gpu: logger.child("gpu", tinylog::Level::Trace),
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
}

/// The shimmer emulator.
pub struct Emulator {
    /// The state of the system.
    psx: PSX,

    /// The GPU command interpreter.
    gpu_interpreter: gpu::Interpreter,
    /// The DMA executor.
    dma_executor: dma::Executor,
}

impl Emulator {
    /// Creates a new [`Emulator`].
    pub fn with_bios(bios: Vec<u8>, logger: Logger) -> (Self, Receiver<gpu::renderer::Action>) {
        let (gpu_interpreter, receiver) = gpu::Interpreter::new();
        let mut e = Self {
            psx: PSX {
                scheduler: Scheduler::default(),
                loggers: Loggers::new(logger),

                memory: mem::Memory::with_bios(bios).expect("BIOS should fit"),
                timers: timers::Timers::default(),
                dma: dma::Controller::default(),
                cpu: cpu::Cpu::default(),
                cop0: cop0::Cop0::default(),
                interrupts: interrupts::Controller::default(),
                gpu: gpu::Gpu::default(),
            },
            dma_executor: dma::Executor::default(),
            gpu_interpreter,
        };

        e.psx.scheduler.schedule(Event::Cpu, 0);
        e.psx.scheduler.schedule(Event::VSync, 0);
        e.psx.scheduler.schedule(Event::Timer2, 0);

        (e, receiver)
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

    /// Executes a single system cycle.
    pub fn cycle(&mut self) {
        self.psx.scheduler.advance();

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
                Event::VSync => {
                    self.psx
                        .gpu
                        .status
                        .set_interlace_odd(!self.psx.gpu.status.interlace_odd());
                    self.psx.interrupts.status.request(Interrupt::VBlank);
                    self.psx
                        .scheduler
                        .schedule(Event::VSync, u64::from(self.psx.gpu.cycles_per_vblank()));
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
            }
        }
    }

    #[inline(always)]
    pub fn cycle_for(&mut self, cycles: u64) {
        for _ in 0..cycles {
            self.cycle();
        }
    }
}
