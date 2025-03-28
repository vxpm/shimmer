//! The event scheduler of the [`PSX`](super::PSX).

use crate::{cdrom, sio0, timers};

/// Possible schedule events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
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
    /// Update timers.
    Timer(timers::Event),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScheduledEvent {
    time: u64,
    event: Event,
}

/// The event scheduler of the [`PSX`](super::PSX).
///
/// The scheduler is responsible for keeping track of how many cycles have elapsed and what should
/// happen next.
#[derive(Debug)]
pub struct Scheduler {
    /// How many cycles have been executed since the start.
    elapsed: u64,
    /// Scheduled events.
    scheduled: Vec<ScheduledEvent>,
    /// The time at which the last scheduled event will happen.
    last_scheduled_time: u64,
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
            last_scheduled_time: u64::MAX,
        };

        scheduler.schedule(Event::VBlank, 0);
        scheduler.schedule(Event::Timer(timers::Event::Setup), 0);

        scheduler
    }

    #[inline(always)]
    pub fn schedule(&mut self, event: Event, after: u64) {
        self.last_scheduled_time = self.elapsed + after;
        self.scheduled.push(ScheduledEvent {
            event,
            time: self.last_scheduled_time,
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
            .min_by_key(|e| e.time)
            .map(|e| e.time - self.elapsed)
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Option<Event> {
        self.scheduled
            .iter()
            .position(|e| e.time <= self.elapsed)
            .map(|i| self.scheduled.swap_remove(i).event)
    }

    #[inline(always)]
    pub fn elapsed(&self) -> u64 {
        self.elapsed
    }

    #[inline(always)]
    pub fn last_scheduled_time(&self) -> u64 {
        self.last_scheduled_time
    }
}
