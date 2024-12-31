use crate::State;
use crossbeam::sync::Parker;
use shimmer_core::cpu::Reg;
use std::{
    sync::{atomic::Ordering, Arc},
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
            let taken = 2048.min(cycles_left);
            cycles_left -= taken;

            for _ in 0..taken {
                exclusive.psx.cycle();

                if exclusive.psx.cpu.to_exec().1 == 0xB0 {
                    let call = exclusive.psx.cpu.regs().read(Reg::R9);
                    if call == 0x3D {
                        let char = exclusive.psx.cpu.regs().read(Reg::A0);
                        if let Ok(char) = char::try_from(char) {
                            exclusive.terminal_output.push(char);
                        }
                    }
                }

                if exclusive
                    .controls
                    .breakpoints
                    .contains(&exclusive.psx.cpu.to_exec().1.value())
                {
                    exclusive.controls.running = false;
                    break;
                }
            }

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
