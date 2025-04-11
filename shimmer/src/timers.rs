use shimmer_core::interrupts::Interrupt;
use tinylog::Logger;

use crate::{PSX, scheduler};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Setup,
    Timer1,
    Timer2,
}

#[derive(Debug)]
pub struct Timers {
    _logger: Logger,
}

impl Timers {
    pub fn new(logger: Logger) -> Self {
        Self { _logger: logger }
    }

    fn tick_timer1(&mut self, psx: &mut PSX) {
        let timer1 = &mut psx.timers.timer1;
        if !timer1.should_tick() {
            psx.scheduler.schedule(
                scheduler::Event::Timer(Event::Timer2),
                timer1.cycles_per_tick(),
            );
            return;
        }

        let old_value = timer1.value;
        timer1.value = timer1.value.wrapping_add(1);

        if timer1.value == 0xFFFF {
            timer1.mode.set_reached_max(true);
            if timer1.mode.irq_at_max() && timer1.can_raise_irq() {
                timer1.update_no_irq();
                psx.interrupts.status.request(Interrupt::Timer2);
            }
        }

        if timer1.value == timer1.target {
            timer1.mode.set_reached_target(true);
            if timer1.mode.irq_when_at_target() && timer1.can_raise_irq() {
                timer1.update_no_irq();
                psx.interrupts.status.request(Interrupt::Timer2);
            }
        } else if old_value == timer1.target && timer1.mode.reset_at_target() {
            timer1.value = 0;
        }

        psx.scheduler.schedule(
            scheduler::Event::Timer(Event::Timer2),
            timer1.cycles_per_tick(),
        );
    }

    fn tick_timer2(&mut self, psx: &mut PSX) {
        let timer2 = &mut psx.timers.timer2;
        if !timer2.should_tick() {
            psx.scheduler.schedule(
                scheduler::Event::Timer(Event::Timer2),
                timer2.cycles_per_tick(),
            );
            return;
        }

        let old_value = timer2.value;
        timer2.value = timer2.value.wrapping_add(1);

        if timer2.value == 0xFFFF {
            timer2.mode.set_reached_max(true);
            if timer2.mode.irq_at_max() && timer2.can_raise_irq() {
                timer2.update_no_irq();
                psx.interrupts.status.request(Interrupt::Timer2);
            }
        }

        if timer2.value == timer2.target {
            timer2.mode.set_reached_target(true);
            if timer2.mode.irq_when_at_target() && timer2.can_raise_irq() {
                timer2.update_no_irq();
                psx.interrupts.status.request(Interrupt::Timer2);
            }
        } else if old_value == timer2.target && timer2.mode.reset_at_target() {
            timer2.value = 0;
        }

        psx.scheduler.schedule(
            scheduler::Event::Timer(Event::Timer2),
            timer2.cycles_per_tick(),
        );
    }

    pub fn update(&mut self, psx: &mut PSX, event: Event) {
        match event {
            Event::Setup => {
                psx.timers.timer1.mode.set_no_irq(true);
                psx.timers.timer2.mode.set_no_irq(true);

                psx.scheduler.schedule(
                    scheduler::Event::Timer(Event::Timer1),
                    psx.timers.timer1.cycles_per_tick(),
                );
                psx.scheduler.schedule(
                    scheduler::Event::Timer(Event::Timer2),
                    psx.timers.timer2.cycles_per_tick(),
                );
            }
            Event::Timer1 => self.tick_timer1(psx),
            Event::Timer2 => self.tick_timer2(psx),
        }
    }
}
