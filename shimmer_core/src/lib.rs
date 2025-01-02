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

enum Event {
    VSync,
}

struct ScheduledEvent {
    event: Event,
    happens_in: u64,
}

#[derive(Default)]
struct Scheduler {
    scheduled: Vec<ScheduledEvent>,
}

impl Scheduler {
    pub fn schedule(&mut self, event: Event, after: u64) {
        self.scheduled.push(ScheduledEvent {
            event,
            happens_in: after,
        });

        self.scheduled
            .sort_unstable_by_key(|e| std::cmp::Reverse(e.happens_in));
    }

    pub fn advance(&mut self, cycles: u64) {
        self.scheduled
            .iter_mut()
            .for_each(|e| e.happens_in = e.happens_in.saturating_sub(cycles));
    }

    pub fn peek(&self) -> Option<&ScheduledEvent> {
        self.scheduled.last()
    }

    pub fn pop(&mut self) -> Option<ScheduledEvent> {
        self.scheduled.pop()
    }
}

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
    scheduler: Scheduler,

    pub memory: mem::Memory,
    pub cpu: cpu::State,
    pub cop0: cop0::State,
    pub gpu: gpu::State,

    pub loggers: Loggers,
}

impl PSX {
    /// Creates a new [`PSX`].
    pub fn with_bios(bios: Vec<u8>, logger: Logger) -> Self {
        let mut value = Self {
            scheduler: Scheduler::default(),
            memory: mem::Memory::with_bios(bios).expect("BIOS should fit"),
            cpu: cpu::State::default(),
            cop0: cop0::State::default(),
            gpu: gpu::State::default(),
            loggers: Loggers::new(logger),
        };

        value.scheduler.schedule(Event::VSync, 0);
        value
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

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::VSync => {
                let bus = self.bus();
                bus.cop0.interrupt_status.request(cop0::Interrupt::VBlank);

                self.scheduler
                    .schedule(Event::VSync, self.gpu.cycles_per_vblank() as u64);
            }
        }
    }

    pub fn cycle_for(&mut self, cycles: u64) {
        let mut cycles_left = cycles;
        loop {
            if self
                .scheduler
                .peek()
                .map(|e| e.happens_in > cycles_left)
                .unwrap_or(true)
            {
                let bus = self.bus();
                let mut interpreter = cpu::Interpreter::new(bus);
                for _ in 0..cycles_left {
                    interpreter.cycle();
                }

                self.scheduler.advance(cycles_left);
                break;
            }

            let next_event = self.scheduler.pop().unwrap();
            let bus = self.bus();
            let mut interpreter = cpu::Interpreter::new(bus);
            for _ in 0..next_event.happens_in {
                interpreter.cycle();
            }

            cycles_left -= next_event.happens_in;
            self.scheduler.advance(next_event.happens_in);
            self.handle_event(next_event.event);
        }
    }
}
