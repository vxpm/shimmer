#![feature(inline_const_pat)]
#![feature(unbounded_shifts)]
#![feature(debug_closure_helpers)]

pub mod cpu;
pub mod mem;
mod util;

use cpu::cop0;

pub struct PSX {
    pub memory: mem::Memory,
    pub cpu: cpu::State,
    pub cop0: cop0::State,
}

impl PSX {
    /// Creates a new [`PSX`].
    pub fn with_bios(bios: Vec<u8>) -> Self {
        Self {
            memory: mem::Memory::with_bios(bios).expect("BIOS should fit"),
            cpu: cpu::State::default(),
            cop0: cop0::State::default(),
        }
    }

    pub fn bus(&mut self) -> mem::Bus {
        mem::Bus {
            memory: &mut self.memory,
            cpu: &mut self.cpu,
            cop0: &mut self.cop0,
        }
    }

    pub fn cycle(&mut self) {
        let bus = mem::Bus {
            memory: &mut self.memory,
            cpu: &mut self.cpu,
            cop0: &mut self.cop0,
        };

        let mut interpreter = cpu::Interpreter::new(bus);
        interpreter.cycle();
    }
}
