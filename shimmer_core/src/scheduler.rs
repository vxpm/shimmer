//! The event scheduler of the [`PSX`](super::PSX).

use crate::{cdrom, sio0};

/// Possible schedule events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Execute the next CPU instruction.
    Cpu,
    /// Fire a VBlank.
    VBlank,
    /// Update the GPU state machine.
    Gpu,
    /// Update the DMA state machine and possibly start a transfer.
    DmaUpdate,
    /// Advance the currently ongoing DMA transfer.
    DmaAdvance,
    /// Update the CDROM state machine.
    Cdrom(cdrom::Event),
    /// Update the SIO state machine.
    Sio(sio0::Event),
    /// Advance Timer1.
    Timer1,
    /// Advance Timer2.
    Timer2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScheduledEvent {
    happens_at: u64,
    event: Event,
}

/// The event scheduler of the [`PSX`](super::PSX).
///
/// The scheduler is responsible for keeping track of how many cycles have elapsed and what should
/// happen next.
#[derive(Debug)]
pub struct Scheduler {
    elapsed: u64,
    scheduled: Vec<ScheduledEvent>,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler {
    pub fn new() -> Self {
        let mut scheduler = Self {
            elapsed: 0,
            scheduled: Vec::with_capacity(16),
        };

        scheduler.schedule(Event::Cpu, 0);
        scheduler.schedule(Event::VBlank, 0);
        scheduler.schedule(Event::Timer1, 0);
        scheduler.schedule(Event::Timer2, 0);

        scheduler
    }

    #[inline(always)]
    pub fn schedule(&mut self, event: Event, after: u64) {
        self.scheduled.push(ScheduledEvent {
            event,
            happens_at: self.elapsed + after,
        });
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.scheduled.len()
    }

    #[inline(always)]
    pub fn advance(&mut self, count: u64) {
        self.elapsed += count;
    }

    #[inline(always)]
    pub fn until_next(&self) -> Option<u64> {
        self.scheduled
            .iter()
            .min_by_key(|e| e.happens_at)
            .map(|e| e.happens_at - self.elapsed)
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Option<Event> {
        self.scheduled
            .iter()
            .position(|e| e.happens_at <= self.elapsed)
            .map(|i| self.scheduled.swap_remove(i).event)
    }

    #[inline(always)]
    pub fn elapsed(&self) -> u64 {
        self.elapsed
    }
}
