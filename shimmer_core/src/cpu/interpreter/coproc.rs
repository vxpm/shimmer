use super::{DEFAULT_CYCLE_COUNT, Interpreter};
use crate::cpu::{COP, RegLoad, instr::Instruction, interpreter::Exception};
use tinylog::warn;

impl Interpreter<'_> {
    /// `copn_rd = rt`
    pub fn mtc(&mut self, instr: Instruction) -> u64 {
        let rt = self.psx.cpu.regs.read(instr.rt());
        let system_status = self.psx.cop0.regs.system_status();

        match instr.cop() {
            COP::COP0 => {
                self.psx.cop0.load_delay_slot = Some(RegLoad {
                    reg: instr.rd(),
                    value: rt,
                })
            }
            COP::COP1 if system_status.cop1_enabled() => {}
            // TODO: remove stub
            COP::COP2 if system_status.cop2_enabled() => {
                warn!(self.psx.loggers.cpu, "mtc to cop2 stubbed")
            }
            COP::COP3 if system_status.cop3_enabled() => {}
            _ => self.trigger_exception(Exception::CopUnusable),
        }

        DEFAULT_CYCLE_COUNT
    }

    /// `rt = copn_rd`
    pub fn mfc(&mut self, instr: Instruction) -> u64 {
        let system_status = self.psx.cop0.regs.system_status();
        let rd = match instr.cop() {
            COP::COP0 => self.psx.cop0.regs.read(instr.rd()),
            COP::COP1 if system_status.cop1_enabled() => return DEFAULT_CYCLE_COUNT,
            // TODO: remove stub
            COP::COP2 if system_status.cop2_enabled() => {
                warn!(self.psx.loggers.cpu, "mfc to cop2 stubbed");
                0
            }
            COP::COP3 if system_status.cop3_enabled() => return DEFAULT_CYCLE_COUNT,
            _ => {
                self.trigger_exception(Exception::CopUnusable);
                return DEFAULT_CYCLE_COUNT;
            }
        };

        self.cancel_load(instr.rt());
        self.psx.cpu.load_delay_slot = Some(RegLoad {
            reg: instr.rt(),
            value: rd,
        });

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
