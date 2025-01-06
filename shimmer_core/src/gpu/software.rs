use tinylog::debug;

use super::instr::{DisplayInstruction, RenderingInstruction};
use crate::mem::Bus;

pub struct Renderer {}

impl Renderer {
    pub fn exec(&mut self, bus: &mut Bus, instr: RenderingInstruction) {
        debug!(bus.loggers.gpu, "received instr: {instr:?}");
    }

    pub fn exec_display(&mut self, bus: &mut Bus, instr: DisplayInstruction) {
        debug!(bus.loggers.gpu, "received instr: {instr:?}");
    }
}

