use super::{DEFAULT_DELAY, Interpreter};
use crate::cpu::{cop0::Exception, instr::Instruction};

impl Interpreter<'_> {
    pub fn syscall(&mut self, _instr: Instruction) -> u64 {
        self.trigger_exception(Exception::Syscall);
        DEFAULT_DELAY
    }

    pub fn breakpoint(&mut self, _instr: Instruction) -> u64 {
        self.trigger_exception(Exception::Breakpoint);
        DEFAULT_DELAY
    }
}
