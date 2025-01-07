#![feature(inline_const_pat)]
#![feature(unbounded_shifts)]
#![feature(debug_closure_helpers)]
#![feature(let_chains)]

pub mod cpu;
pub mod dma;
pub mod exe;
pub mod gpu;
pub mod kernel;
pub mod mem;
pub mod timers;

mod scheduler;
mod util;

use cpu::cop0;
use scheduler::{Event, Scheduler};
use tinylog::Logger;

pub use binrw;

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

/// The state of the PSX.
pub struct PSX {
    pub scheduler: Scheduler,
    pub loggers: Loggers,

    pub memory: mem::Memory,
    pub timers: timers::State,
    pub dma: dma::State,
    pub cpu: cpu::State,
    pub cop0: cop0::State,
    pub gpu: gpu::State,
}

pub struct Emulator {
    psx: PSX,
    renderer: gpu::software::Renderer,
}

impl Emulator {
    /// Creates a new [`PSX`].
    pub fn with_bios(bios: Vec<u8>, logger: Logger) -> Self {
        let mut e = Self {
            psx: PSX {
                scheduler: Scheduler::default(),
                loggers: Loggers::new(logger),

                memory: mem::Memory::with_bios(bios).expect("BIOS should fit"),
                timers: timers::State::default(),
                dma: dma::State::default(),
                cpu: cpu::State::default(),
                cop0: cop0::State::default(),
                gpu: gpu::State::default(),
            },

            renderer: gpu::software::Renderer {},
        };

        e.psx.scheduler.schedule(Event::Cpu, 0);
        e.psx.scheduler.schedule(Event::VSync, 0);
        e.psx.scheduler.schedule(Event::Timer2, 0);
        e.psx.scheduler.schedule(Event::Gpu, 0);
        e.psx.scheduler.schedule(Event::Dma, 0);

        e
    }

    #[inline(always)]
    pub fn psx(&mut self) -> &PSX {
        &self.psx
    }

    #[inline(always)]
    pub fn psx_mut(&mut self) -> &mut PSX {
        &mut self.psx
    }

    pub fn cycle(&mut self) {
        self.psx.scheduler.advance(1);
        while let Some(e) = self.psx.scheduler.pop() {
            match e {
                Event::Cpu => {
                    let mut interpreter = cpu::Interpreter::new(self.psx_mut());
                    let cycles = interpreter.exec_next();

                    self.psx.scheduler.schedule(Event::Cpu, cycles);
                }
                Event::VSync => {
                    let bus = self.psx_mut();
                    bus.cop0.interrupt_status.request(cop0::Interrupt::VBlank);

                    self.psx
                        .scheduler
                        .schedule(Event::VSync, u64::from(self.psx.gpu.cycles_per_vblank()));
                }
                Event::Timer2 => {
                    let cycles = self.psx.timers.timer2.tick();
                    self.psx.scheduler.schedule(Event::Timer2, cycles);
                }
                Event::Gpu => {
                    while !self.psx.gpu.queue.is_empty() {
                        let instr = self.psx.gpu.queue.pop_front().unwrap();
                        self.renderer.exec(&mut self.psx, instr);
                    }

                    self.psx.scheduler.schedule(Event::Gpu, 2);
                }
                Event::Dma => {
                    dma::check_transfers(&mut self.psx);
                    self.psx.scheduler.schedule(Event::Dma, 16);
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
