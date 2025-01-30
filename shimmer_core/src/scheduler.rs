//! The event scheduler of the [`PSX`](super::PSX).

use crate::cdrom;

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
    /// Advance Timer1.
    Timer1,
    /// Advance Timer2.
    Timer2,
}

impl Event {
    pub fn priority(self) -> u8 {
        match self {
            Event::Cpu => 0,
            Event::VBlank => 1,
            Event::Gpu => 2,
            Event::DmaUpdate => 3,
            Event::DmaAdvance => 4,
            Event::Cdrom(_) => 5,
            Event::Timer1 => 6,
            Event::Timer2 => 6,
        }
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority().cmp(&other.priority())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
        Self {
            elapsed: 0,
            scheduled: vec![
                ScheduledEvent {
                    happens_at: 0,
                    event: Event::Cpu,
                },
                ScheduledEvent {
                    happens_at: 0,
                    event: Event::VBlank,
                },
                ScheduledEvent {
                    happens_at: 0,
                    event: Event::Timer1,
                },
                ScheduledEvent {
                    happens_at: 0,
                    event: Event::Timer2,
                },
            ],
        }
    }

    #[inline(always)]
    pub fn schedule(&mut self, event: Event, after: u64) {
        let event = ScheduledEvent {
            event,
            happens_at: self.elapsed + after,
        };

        let mut pos = self.scheduled.len();
        for i in 0..pos {
            let elem = unsafe { self.scheduled.get_unchecked(i) };
            if elem <= &event {
                pos = i;
                break;
            }
        }

        self.scheduled.insert(pos, event);
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.scheduled.len()
    }

    #[inline(always)]
    pub fn advance(&mut self) {
        self.elapsed += 1;
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Option<Event> {
        if self
            .scheduled
            .last()
            .is_some_and(|e| e.happens_at.saturating_sub(self.elapsed) == 0)
        {
            self.scheduled.pop().map(|e| e.event)
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn elapsed(&self) -> u64 {
        self.elapsed
    }
}
