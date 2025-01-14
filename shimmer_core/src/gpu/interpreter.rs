mod display;
mod rendering;

use super::cmd::{DisplayCommand, RenderingCommand, rendering::LineCmd};
use crate::{
    PSX,
    gpu::cmd::rendering::{
        CoordPacket, ShadingMode, SizePacket, VertexColorPacket, VertexPositionPacket,
    },
    scheduler::Event,
};
use tinylog::debug;

#[derive(Debug, Default)]
enum InterpreterInner {
    #[default]
    Idle,
    /// Waiting for enough data to complete a CPU to VRAM blit
    CpuToVramBlit {
        _dest: CoordPacket,
        size: SizePacket,
    },
    PolyLine {
        cmd: LineCmd,
        received: u32,
    },
}

/// A GPU packet interpreter.
#[derive(Debug, Default)]
pub struct Interpreter(InterpreterInner);

impl Interpreter {
    fn exec_queued_render(&mut self, psx: &mut PSX) {
        match &mut self.0 {
            InterpreterInner::Idle => {
                if let Some(packet) = psx.gpu.render_queue.front() {
                    let cmd = RenderingCommand::from_bits(*packet);
                    if psx.gpu.render_queue.len() <= cmd.args() {
                        debug!(
                            psx.loggers.gpu,
                            "{cmd:?} is waiting for {} arguments (has {})",
                            cmd.args(),
                            psx.gpu.render_queue.len() - 1,
                        );
                        return;
                    }

                    psx.gpu.render_queue.pop_front();
                    self.exec_render(psx, cmd);
                    self.exec_queued_render(psx);
                }
            }
            InterpreterInner::CpuToVramBlit { _dest, size } => {
                let packets = (size.width() * size.height() + 1) / 2;
                if psx.gpu.render_queue.len() < packets as usize {
                    return;
                }

                for _ in 0..packets {
                    // TODO: perform blit
                    let _packet = psx.gpu.render_queue.pop_front().unwrap();
                }

                self.0 = InterpreterInner::Idle;

                psx.gpu.status.set_ready_to_send_vram(false);
                psx.scheduler.schedule(Event::DmaUpdate, 0);

                self.exec_queued_render(psx);
            }
            InterpreterInner::PolyLine { cmd, received } => {
                let Some(front) = psx.gpu.render_queue.front() else {
                    return;
                };

                if *received >= 2 && (front & 0xF000_F000 == 0x5000_5000) {
                    debug!(psx.loggers.gpu, "exiting polyline mode",);
                    psx.gpu.render_queue.pop_front();
                    self.0 = InterpreterInner::Idle;
                    self.exec_queued_render(psx);
                    return;
                }

                match (cmd.shading_mode(), psx.gpu.render_queue.len()) {
                    (ShadingMode::Flat, _) => {
                        debug!(
                            psx.loggers.gpu,
                            "vertex: {:?}",
                            VertexPositionPacket::from_bits(
                                psx.gpu.render_queue.pop_front().unwrap()
                            )
                        );

                        *received += 1;
                    }
                    (ShadingMode::Gouraud, x) if x >= 2 => {
                        debug!(
                            psx.loggers.gpu,
                            "gouraud: {:?}",
                            VertexColorPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap())
                        );

                        debug!(
                            psx.loggers.gpu,
                            "vertex: {:?}",
                            VertexPositionPacket::from_bits(
                                psx.gpu.render_queue.pop_front().unwrap()
                            )
                        );

                        *received += 1;
                    }
                    _ => (),
                }
            }
        }
    }

    fn exec_queued_display(&mut self, psx: &mut PSX) {
        while let Some(packet) = psx.gpu.display_queue.pop_front() {
            let cmd = DisplayCommand::from_bits(packet);
            self.exec_display(psx, cmd);
        }
    }

    /// Executes all queued GPU commands.
    pub fn exec_queued(&mut self, psx: &mut PSX) {
        self.exec_queued_display(psx);
        self.exec_queued_render(psx);
    }
}
