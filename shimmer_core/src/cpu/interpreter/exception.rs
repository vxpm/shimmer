use super::Interpreter;
use crate::cpu::{cop0::Exception, instr::Instruction};

impl Interpreter<'_> {
    pub fn syscall(&mut self, _instr: Instruction) {
        dbg!(self.current_addr);
        self.trigger_exception(Exception::Syscall);
        self.bus.cpu.regs.pc = self.bus.cpu.regs.pc.saturating_sub(4);
    }
}
