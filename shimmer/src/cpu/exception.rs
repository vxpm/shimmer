use super::{DEFAULT_DELAY, Interpreter};
use crate::PSX;
use shimmer_core::cpu::{cop0::Exception, instr::Instruction};

impl Interpreter {
    pub fn syscall(&mut self, psx: &mut PSX, _instr: Instruction) -> u64 {
        self.trigger_exception(psx, Exception::Syscall);
        DEFAULT_DELAY
    }

    pub fn breakpoint(&mut self, psx: &mut PSX, _instr: Instruction) -> u64 {
        self.trigger_exception(psx, Exception::Breakpoint);
        DEFAULT_DELAY
    }
}
