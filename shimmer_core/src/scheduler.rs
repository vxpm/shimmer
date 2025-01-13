#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Event {
    Cpu,
    VSync,
    Timer2,
    Gpu,
    DmaUpdate,
    DmaAdvance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ScheduledEvent {
    happens_at: u64,
    event: Event,
}

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
