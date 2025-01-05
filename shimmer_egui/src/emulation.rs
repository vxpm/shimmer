use crate::State;
use crossbeam::sync::Parker;
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

pub const CPU_FREQ: u32 = 33_870_000;

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

        let cycles_to_run = CPU_FREQ as f64 * time_behind.as_secs_f64();
        let full_cycles_to_run = cycles_to_run as u64;

        let mut cycles_left = full_cycles_to_run;
        while cycles_left > 0 {
            let taken = 4096.min(cycles_left);
            cycles_left -= taken;

            exclusive.psx.cycle_for(taken);

            // for _ in 0..taken {
            //     // if exclusive
            //     //     .controls
            //     //     .breakpoints
            //     //     .contains(&exclusive.psx.cpu.to_exec().1.value())
            //     // {
            //     //     exclusive.controls.running = false;
            //     //     state.shared.should_advance.store(false, Ordering::Relaxed);
            //     //     break;
            //     // }
            // }

            let should_advance = state.shared.should_advance.load(Ordering::Relaxed);
            if !should_advance {
                break;
            }
        }

        let emulated_cycles = full_cycles_to_run - cycles_left;
        exclusive.timing.emulated_time +=
            Duration::from_secs_f64(emulated_cycles as f64 / CPU_FREQ as f64);
    }
}
