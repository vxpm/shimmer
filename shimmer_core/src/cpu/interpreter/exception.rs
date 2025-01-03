use super::Interpreter;
use crate::cpu::{cop0::Exception, instr::Instruction};

impl Interpreter<'_> {
    pub fn syscall(&mut self, _instr: Instruction) {
        self.trigger_exception(Exception::Syscall);
        self.bus.cpu.regs.pc = self.bus.cpu.regs.pc.wrapping_sub(4);
    }
}
