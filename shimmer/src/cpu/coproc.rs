use super::{DEFAULT_DELAY, Interpreter};
use crate::PSX;
use shimmer_core::cpu::{COP, RegLoad, cop0::Exception, instr::Instruction};

impl Interpreter {
    /// `copn_rd_data = rt`
    pub fn mtc(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rt = psx.cpu.regs.read(instr.rt());
        let system_status = psx.cop0.regs.system_status();

        match instr.cop() {
            COP::COP0 => {
                psx.cop0.regs.write(instr.cop0_rd(), rt);
            }
            COP::COP1 if system_status.cop1_enabled() => {}
            COP::COP2 if system_status.cop2_enabled() => {
                psx.gte.regs.write(instr.gte_data_rd().into(), rt);
            }
            COP::COP3 if system_status.cop3_enabled() => {}
            _ => self.trigger_exception(psx, Exception::CopUnusable),
        }

        DEFAULT_DELAY
    }

    /// `rt = copn_rd_data`
    pub fn mfc(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let system_status = psx.cop0.regs.system_status();
        let rd = match instr.cop() {
            COP::COP0 => psx.cop0.regs.read(instr.cop0_rd()),
            COP::COP1 if system_status.cop1_enabled() => return DEFAULT_DELAY,
            COP::COP2 if system_status.cop2_enabled() => {
                psx.gte.regs.read(instr.gte_data_rd().into())
            }
            COP::COP3 if system_status.cop3_enabled() => return DEFAULT_DELAY,
            _ => {
                self.trigger_exception(psx, Exception::CopUnusable);
                return DEFAULT_DELAY;
            }
        };

        self.cancel_load(instr.rt());
        self.load_delay_slot = Some(RegLoad {
            reg: instr.rt(),
            value: rd,
        });

        DEFAULT_DELAY
    }

    /// `copn_rd_control = rt`
    pub fn ctc(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let rt = psx.cpu.regs.read(instr.rt());
        let system_status = psx.cop0.regs.system_status();

        match instr.cop() {
            COP::COP0 => {}
            COP::COP1 if system_status.cop1_enabled() => {}
            COP::COP2 if system_status.cop2_enabled() => {
                psx.gte.regs.write(instr.gte_control_rd().into(), rt);
            }
            COP::COP3 if system_status.cop3_enabled() => {}
            _ => self.trigger_exception(psx, Exception::CopUnusable),
        }

        DEFAULT_DELAY
    }

    /// `rt = copn_rd_control`
    pub fn cfc(&mut self, psx: &mut PSX, instr: Instruction) -> u64 {
        let system_status = psx.cop0.regs.system_status();
        let rd = match instr.cop() {
            COP::COP0 => psx.cop0.regs.read(instr.cop0_rd()),
            COP::COP1 if system_status.cop1_enabled() => return DEFAULT_DELAY,
            COP::COP2 if system_status.cop2_enabled() => {
                psx.gte.regs.read(instr.gte_control_rd().into())
            }
            COP::COP3 if system_status.cop3_enabled() => return DEFAULT_DELAY,
            _ => {
                self.trigger_exception(psx, Exception::CopUnusable);
                return DEFAULT_DELAY;
            }
        };

        self.cancel_load(instr.rt());
        self.load_delay_slot = Some(RegLoad {
            reg: instr.rt(),
            value: rd,
        });

        DEFAULT_DELAY
    }

    /// Prepares a return from an exception.
    pub fn rfe(&mut self, psx: &mut PSX, _instr: Instruction) -> u64 {
        psx.cop0.regs.system_status_mut().restore_from_exception();

        DEFAULT_DELAY
    }
}
