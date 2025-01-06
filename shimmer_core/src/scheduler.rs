use dary_heap::BinaryHeap;

#[derive(Debug, PartialEq, Eq)]
pub enum Event {
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
pub struct Scheduler {
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
    pub fn pop(&mut self) -> Option<Event> {
        if self
            .scheduled
            .peek()
            .is_some_and(|e| e.happens_at.saturating_sub(self.elapsed) == 0)
        {
            self.scheduled.pop().map(|e| e.event)
        } else {
            None
        }
    }
}
