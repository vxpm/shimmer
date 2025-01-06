use tinylog::debug;

use super::instr::Instruction;
use crate::mem::Bus;

pub struct Renderer {}

impl Renderer {
    pub fn exec(&mut self, bus: &mut Bus, instr: Instruction) {
        debug!(bus.loggers.gpu, "received instr: {instr:?}");
    }
}
