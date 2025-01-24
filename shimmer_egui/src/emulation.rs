use crate::State;
use crossbeam::sync::Parker;
use shimmer_core::cpu::FREQUENCY;
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

pub fn run(state: Arc<State>, parker: Parker) {
    loop {
        let should_advance = state.shared.should_advance.load(Ordering::Relaxed);
        if !should_advance {
            parker.park();
            continue;
        }

        let mut exclusive = state.exclusive.lock();
        let time_behind = exclusive
            .timing
            .running_timer
            .elapsed()
            .saturating_sub(exclusive.timing.emulated_time);

        let cycles_to_run = FREQUENCY as f64 * time_behind.as_secs_f64();
        let full_cycles_to_run = cycles_to_run as u64;

        let mut cycles_left = full_cycles_to_run;
        'outer: while cycles_left > 0 {
            let taken = 4096.min(cycles_left);

            for _ in 0..taken {
                exclusive.emulator.cycle();
                cycles_left -= 1;

                let addr = exclusive.emulator.psx().cpu.instr_delay_slot().1.value();
                if exclusive.controls.breakpoints.contains(&addr) {
                    exclusive.controls.running = false;
                    exclusive.timing.running_timer.pause();
                    state.shared.should_advance.store(false, Ordering::Relaxed);
                    break 'outer;
                }
            }

            let should_advance = state.shared.should_advance.load(Ordering::Relaxed);
            if !should_advance {
                break;
            }
        }

        let emulated_cycles = full_cycles_to_run - cycles_left;
        exclusive.timing.emulated_time +=
            Duration::from_secs_f64(emulated_cycles as f64 / FREQUENCY as f64);
    }
}
