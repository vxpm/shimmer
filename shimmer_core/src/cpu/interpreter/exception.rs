use super::Interpreter;
use crate::cpu::{cop0::Exception, instr::Instruction};

impl Interpreter<'_> {
    pub fn syscall(&mut self, _instr: Instruction) {
        self.trigger_exception(Exception::Syscall);
    }

    pub fn breakpoint(&mut self, _instr: Instruction) {
        self.trigger_exception(Exception::Breakpoint);
    }
}
