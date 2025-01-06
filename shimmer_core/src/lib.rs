#![feature(inline_const_pat)]
#![feature(unbounded_shifts)]
#![feature(debug_closure_helpers)]
#![feature(let_chains)]

pub mod cpu;
pub mod exe;
pub mod gpu;
pub mod kernel;
pub mod mem;
pub mod timers;
mod util;

use cpu::cop0;
use std::collections::BinaryHeap;
use tinylog::Logger;

pub use binrw;

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Cpu,
    VSync,
    Timer2,
}

#[derive(Debug)]
struct ScheduledEvent {
    happens_at: u64,
    event: Event,
}

impl PartialEq for ScheduledEvent {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.happens_at == other.happens_at
    }
}

impl Eq for ScheduledEvent {}

impl PartialOrd for ScheduledEvent {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        std::cmp::Reverse(self.happens_at).partial_cmp(&std::cmp::Reverse(other.happens_at))
    }
}
impl Ord for ScheduledEvent {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        std::cmp::Reverse(self.happens_at).cmp(&std::cmp::Reverse(other.happens_at))
    }
}

#[derive(Default)]
struct Scheduler {
    elapsed: u64,
    scheduled: BinaryHeap<ScheduledEvent>,
}

impl Scheduler {
    pub fn schedule(&mut self, event: Event, after: u64) {
        let cycle = self.elapsed + after;
        self.scheduled.push(ScheduledEvent {
            happens_at: cycle,
            event,
        });
    }

    #[inline(always)]
    pub fn advance(&mut self, cycles: u64) {
        self.elapsed += cycles;
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Option<ScheduledEvent> {
        if self
            .scheduled
            .peek()
            .is_some_and(|e| e.happens_at.saturating_sub(self.elapsed) == 0)
        {
            self.scheduled.pop()
        } else {
            None
        }
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
    bus: mem::Bus,
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
        };

        psx.scheduler.schedule(Event::Cpu, 0);
        psx.scheduler
            .schedule(Event::VSync, psx.bus.gpu.cycles_per_vblank() as u64);
        psx.scheduler.schedule(Event::Timer2, 0);

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
            match e.event {
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
