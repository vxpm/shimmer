use super::{DEFAULT_CYCLE_COUNT, Interpreter};
use crate::cpu::{COP, RegLoad, instr::Instruction};
use tinylog::{error, warn};

impl Interpreter<'_> {
    /// `copn_rd = rt`
    pub fn mtc(&mut self, instr: Instruction) -> u64 {
        if let Some(cop) = instr.cop() {
            let rt = self.psx.cpu.regs.read(instr.rt());
            match cop {
                COP::COP0 => self.psx.cop0.to_load = Some((instr.rd(), rt)),
                // TODO: remove stub
                COP::COP2 => warn!(self.psx.loggers.cpu, "mtc to cop2 stubbed"),
            }
        } else {
            error!(self.psx.loggers.cpu, "mtc to unknown cop");
        }

        DEFAULT_CYCLE_COUNT
    }

    /// `rt = copn_rd`
    pub fn mfc(&mut self, instr: Instruction) -> u64 {
        if let Some(cop) = instr.cop() {
            let rd = match cop {
                COP::COP0 => self.psx.cop0.regs.read(instr.rd()),
                // TODO: remove stub
                COP::COP2 => {
                    warn!(self.psx.loggers.cpu, "mfc to cop2 stubbed");
                    0
                }
            };

            self.psx.cpu.load_delay_slot = Some(RegLoad {
                reg: instr.rt(),
                value: rd,
            });
        } else {
            error!(self.psx.loggers.cpu, "mfc to unknown cop");
        }

        DEFAULT_CYCLE_COUNT
    }

    /// Prepares a return from an exception.
    pub fn rfe(&mut self, _instr: Instruction) -> u64 {
        self.psx
            .cop0
            .regs
            .system_status_mut()
            .restore_from_exception();

        DEFAULT_CYCLE_COUNT
    }
}
