//! The event scheduler of the [`PSX`](super::PSX).

/// Possible schedule events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Event {
    /// Execute the next CPU instruction.
    Cpu,
    /// Fire a VSync.
    VBlank,
    /// Update the GPU state machine.
    Gpu,
    /// Update the DMA state machine and possibly start a transfer.
    DmaUpdate,
    /// Advance the currently ongoing DMA transfer.
    DmaAdvance,
    /// Update the CDROM state machine.
    Cdrom,
    /// Advance Timer2.
    Timer2,
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
#[derive(Debug, Default)]
pub struct Scheduler {
    elapsed: u64,
    scheduled: Vec<ScheduledEvent>,
}

impl Scheduler {
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
