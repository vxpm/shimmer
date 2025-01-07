use tinylog::debug;

use super::instr::Instruction;
use crate::PSX;

pub struct Renderer {}

impl Renderer {
    pub fn exec(&mut self, bus: &mut PSX, instr: Instruction) {
        debug!(bus.loggers.gpu, "received instr: {instr:?}");
    }
}
