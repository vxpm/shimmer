use super::Interpreter;
use crate::cpu::{COP, instr::Instruction};
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

            self.bus.cpu.load_delay_slot = Some((instr.rt(), rd));
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

#[cfg(test)]
mod tests {
    use crate::cpu::{
        Reg,
        interpreter::test::{any_cop, any_reg, any_writable_reg, state, test_interpreter},
    };
    use proptest::prelude::*;

    fn any_cop_writable_reg() -> impl Strategy<Value = Reg> {
        any_writable_reg().prop_filter("cannot be cause, EPC or BadVaddr", |r| {
            !matches!(*r, Reg::COP0_CAUSE | Reg::COP0_EPC | Reg::COP0_BAD_VADDR)
        })
    }

    proptest::proptest! {
        #[test]
        fn mtc(state in state(), cop in any_cop(), rd in any_cop_writable_reg(), rt in any_reg()) {
            test_interpreter! {
                interpreter(state) =>
                mtc(cop, rd, rt)
            };

            interpreter.cycle_for(2);

            let cop0_rd = interpreter.bus.cop0.regs.read(rd);
            let rt = interpreter.bus.cpu.regs.read(rt);
            prop_assert_eq!(cop0_rd, rt);
        }

        #[test]
        fn mfc(state in state(), cop in any_cop(), rd in any_reg(), rt in any_writable_reg()) {
            test_interpreter! {
                interpreter(state) =>
                mfc(cop, rd, rt)
            };

            interpreter.cycle_for(2);

            let cop0_rd = interpreter.bus.cop0.regs.read(rd);
            let rt = interpreter.bus.cpu.regs.read(rt);
            prop_assert_eq!(rt, cop0_rd);
        }
    }
}
