#![feature(inline_const_pat)]
#![feature(unbounded_shifts)]
#![feature(debug_closure_helpers)]
#![feature(let_chains)]
#![feature(select_unpredictable)]

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
    pub cpu: Logger,
    pub kernel: Logger,
    pub gpu: Logger,
}

impl Loggers {
    pub fn new(logger: Logger) -> Self {
        Self {
            bus: logger.child("bus", tinylog::Level::Trace),
            cpu: logger.child("cpu", tinylog::Level::Trace),
            kernel: logger.child("kernel", tinylog::Level::Trace),
            gpu: logger.child("gpu", tinylog::Level::Trace),
            root: logger,
        }
    }
}

pub struct PSX {
    scheduler: Scheduler,
    bus: mem::Bus,

    renderer: gpu::software::Renderer,
}

impl PSX {
    /// Creates a new [`PSX`].
    pub fn with_bios(bios: Vec<u8>, logger: Logger) -> Self {
        let mut psx = Self {
            scheduler: Scheduler::default(),
            bus: mem::Bus {
                memory: mem::Memory::with_bios(bios).expect("BIOS should fit"),
                timers: timers::Timers::default(),
                cpu: cpu::State::default(),
                cop0: cop0::State::default(),
                gpu: gpu::State::default(),
                loggers: Loggers::new(logger),
            },

            renderer: gpu::software::Renderer {},
        };

        psx.scheduler.schedule(Event::Cpu, 0);
        psx.scheduler
            .schedule(Event::VSync, psx.bus.gpu.cycles_per_vblank() as u64);
        psx.scheduler.schedule(Event::Timer2, 0);
        psx.scheduler.schedule(Event::Gpu, 0);

        psx
    }

    #[inline(always)]
    pub fn bus(&mut self) -> &mem::Bus {
        &self.bus
    }

    #[inline(always)]
    pub fn bus_mut(&mut self) -> &mut mem::Bus {
        &mut self.bus
    }

    pub fn cycle(&mut self) {
        self.scheduler.advance(1);
        while let Some(e) = self.scheduler.pop() {
            match e {
                Event::Cpu => {
                    let mut interpreter = cpu::Interpreter::new(self.bus_mut());
                    let _cycles = interpreter.cycle();

                    self.scheduler.schedule(Event::Cpu, 2);
                }
                Event::VSync => {
                    let bus = self.bus_mut();
                    bus.cop0.interrupt_status.request(cop0::Interrupt::VBlank);

                    self.scheduler
                        .schedule(Event::VSync, self.bus.gpu.cycles_per_vblank() as u64);
                }
                Event::Timer2 => {
                    let cycles = self.bus.timers.timer2.tick();
                    self.scheduler.schedule(Event::Timer2, cycles);
                }
                Event::Gpu => {
                    while !self.bus.gpu.queue.is_empty() {
                        let instr = self.bus.gpu.queue.pop_front().unwrap();
                        self.renderer.exec(&mut self.bus, instr);
                    }

                    self.scheduler.schedule(Event::Gpu, 2);
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
