use super::Interpreter;
use crate::cpu::{COP, RegLoad, instr::Instruction};
use tinylog::{error, warn};

impl Interpreter<'_> {
    /// `copn_rd = rt`
    pub fn mtc(&mut self, instr: Instruction) {
        if let Some(cop) = instr.cop() {
            let rt = self.bus.cpu.regs.read(instr.rt());
            match cop {
                COP::COP0 => self.bus.cop0.to_load = Some((instr.rd(), rt)),
                // TODO: remove stub
                COP::COP2 => warn!(self.bus.loggers.cpu, "mtc to cop2 stubbed"),
            }
        } else {
            error!(self.bus.loggers.cpu, "mtc to unknown cop");
        }
    }

    /// `rt = copn_rd`
    pub fn mfc(&mut self, instr: Instruction) {
        if let Some(cop) = instr.cop() {
            let rd = match cop {
                COP::COP0 => self.bus.cop0.regs.read(instr.rd()),
                // TODO: remove stub
                COP::COP2 => {
                    warn!(self.bus.loggers.cpu, "mfc to cop2 stubbed");
                    0
                }
            };

            self.bus.cpu.load_delay_slot = Some(RegLoad {
                reg: instr.rt(),
                value: rd,
            });
        } else {
            error!(self.bus.loggers.cpu, "mfc to unknown cop");
        }
    }

    /// Prepares a return from an exception.
    pub fn rfe(&mut self, _instr: Instruction) {
        self.bus
            .cop0
            .regs
            .system_status_mut()
            .restore_from_exception();
    }
}
