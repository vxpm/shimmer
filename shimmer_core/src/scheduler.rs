#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Cpu,
    VSync,
    Timer2,
}

#[derive(Debug, Clone, Copy)]
struct ScheduledEvent {
    happens_at: u64,
    event: Event,
}

#[derive(Default)]
pub struct Scheduler {
    elapsed: u64,
    scheduled: Vec<ScheduledEvent>,
}

impl Scheduler {
    #[inline(always)]
    pub fn schedule(&mut self, event: Event, after: u64) {
        let cycle = self.elapsed + after;

        let mut pos = self.scheduled.len();
        for i in 0..pos {
            let elem = unsafe { self.scheduled.get_unchecked(i) };
            if elem.happens_at <= cycle {
                pos = i;
                break;
            }
        }

        self.scheduled.insert(pos, ScheduledEvent {
            event,
            happens_at: cycle,
        });
    }

    #[inline(always)]
    pub fn advance(&mut self, cycles: u64) {
        self.elapsed += cycles;
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
}
