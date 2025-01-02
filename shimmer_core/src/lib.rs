#![feature(inline_const_pat)]
#![feature(unbounded_shifts)]
#![feature(debug_closure_helpers)]
#![feature(let_chains)]

pub mod cpu;
pub mod exe;
pub mod gpu;
pub mod kernel;
pub mod mem;
mod util;

use cpu::cop0;
use tinylog::Logger;

pub use binrw;

pub struct Loggers {
    pub root: Logger,
    pub bus: Logger,
    pub cpu: Logger,
    pub kernel: Logger,
}

impl Loggers {
    pub fn new(logger: Logger) -> Self {
        Self {
            bus: logger.child("bus", tinylog::Level::Trace),
            cpu: logger.child("cpu", tinylog::Level::Trace),
            kernel: logger.child("kernel", tinylog::Level::Trace),
            root: logger,
        }
    }
}

pub struct PSX {
    pub memory: mem::Memory,
    pub cpu: cpu::State,
    pub cop0: cop0::State,
    pub gpu: gpu::State,
    pub loggers: Loggers,
}

impl PSX {
    /// Creates a new [`PSX`].
    pub fn with_bios(bios: Vec<u8>, logger: Logger) -> Self {
        Self {
            memory: mem::Memory::with_bios(bios).expect("BIOS should fit"),
            cpu: cpu::State::default(),
            cop0: cop0::State::default(),
            gpu: gpu::State::default(),
            loggers: Loggers::new(logger),
        }
    }

    #[inline(always)]
    pub fn bus(&mut self) -> mem::Bus {
        mem::Bus {
            memory: &mut self.memory,
            cpu: &mut self.cpu,
            cop0: &mut self.cop0,
            gpu: &mut self.gpu,
            loggers: &mut self.loggers,
        }
    }

    pub fn cycle(&mut self) {
        let bus = self.bus();
        let mut interpreter = cpu::Interpreter::new(bus);
        interpreter.cycle();
    }
}
