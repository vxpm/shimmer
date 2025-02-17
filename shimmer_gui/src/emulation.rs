use crate::State;
use crossbeam::sync::Parker;
use parking_lot::Mutex;
use shimmer::core::cpu::FREQUENCY;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

pub fn run(should_advance: Arc<AtomicBool>, state: Arc<Mutex<State>>, parker: Parker) {
    loop {
        let stop = !should_advance.load(Ordering::Relaxed);
        if stop {
            parker.park();
            continue;
        }

        let mut exclusive = state.lock();
        let time_behind = exclusive
            .timing
            .running_timer
            .elapsed()
            .saturating_sub(exclusive.timing.emulated_time);

        let cycles_to_run = FREQUENCY as f64 * time_behind.as_secs_f64();
        let full_cycles_to_run = cycles_to_run as u64;

        const CYCLE_GROUP: u64 = 4096;
        let mut cycles_left = full_cycles_to_run;
        while cycles_left > 0 {
            let taken = CYCLE_GROUP.min(cycles_left);
            cycles_left -= taken;

            exclusive.emulator.cycle_for(taken);

            let stop = !should_advance.load(Ordering::Relaxed);
            if stop {
                break;
            }
        }

        let emulated_cycles = full_cycles_to_run - cycles_left;
        exclusive.timing.emulated_time +=
            Duration::from_secs_f64(emulated_cycles as f64 / FREQUENCY as f64);
    }
}
