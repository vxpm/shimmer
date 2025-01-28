mod display;
mod rendering;

use crate::{
    PSX,
    gpu::{
        cmd::{
            DisplayCommand, RenderingCommand,
            rendering::{
                CoordPacket, LineCmd, ShadingMode, SizePacket, VertexColorPacket,
                VertexPositionPacket,
            },
        },
        renderer::{Command, CopyToVram},
    },
    interrupts::Interrupt,
    scheduler::Event,
};
use bitos::integer::u10;
use tinylog::debug;

use super::renderer::Renderer;

/// The state of the interpreter.
#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    /// Waiting for enough data to complete a CPU to VRAM blit
    CpuToVramBlit {
        dest: CoordPacket,
        size: SizePacket,
    },
    PolyLine {
        cmd: LineCmd,
        received: u32,
    },
}

/// A GPU packet interpreter.
pub struct Interpreter {
    inner: State,
    renderer: Box<dyn Renderer>,
}

impl Interpreter {
    pub fn new(renderer: impl Renderer + 'static) -> Self {
        Self {
            inner: State::default(),
            renderer: Box::new(renderer),
        }
    }

    fn exec_queued_render(&mut self, psx: &mut PSX) {
        loop {
            match &mut self.inner {
                State::Idle => {
                    let Some(packet) = psx.gpu.render_queue.front() else {
                        return;
                    };

                    let cmd = RenderingCommand::from_bits(*packet);
                    if psx.gpu.render_queue.len() <= cmd.args() {
                        debug!(
                            psx.loggers.gpu,
                            "{cmd:?} is waiting for arguments (has {}/{})",
                            psx.gpu.render_queue.len() - 1,
                            cmd.args(),
                        );
                        return;
                    }

                    psx.gpu.render_queue.pop_front();
                    self.exec_render(psx, cmd);
                }
                State::CpuToVramBlit { dest, size } => {
                    let real_width = if size.width() == 0 {
                        0x400
                    } else {
                        ((size.width() - 1) & 0x3FF) + 1
                    };

                    let real_height = if size.height() == 0 {
                        0x200
                    } else {
                        ((size.height() - 1) & 0x1FF) + 1
                    };

                    let count = (real_width as u32 * real_height as u32).div_ceil(2);
                    if psx.gpu.render_queue.len() < count as usize {
                        return;
                    }

                    let mut data = Vec::with_capacity(count as usize * 4);
                    for _ in 0..count {
                        data.extend(
                            psx.gpu
                                .render_queue
                                .pop_front()
                                .unwrap()
                                .to_le_bytes()
                                .into_iter(),
                        );
                    }

                    self.renderer.exec(Command::CopyToVram(CopyToVram {
                        x: dest.x() & 0x3FF,
                        y: dest.y() & 0x1FF,
                        width: real_width,
                        height: real_height,
                        data,
                    }));

                    self.inner = State::Idle;

                    psx.gpu.status.set_ready_to_send_vram(false);
                    psx.scheduler.schedule(Event::DmaUpdate, 0);
                }
                State::PolyLine { cmd, received } => {
                    let Some(front) = psx.gpu.render_queue.front() else {
                        return;
                    };

                    if *received >= 2 && (front & 0xF000_F000 == 0x5000_5000) {
                        debug!(psx.loggers.gpu, "exiting polyline mode",);
                        psx.gpu.render_queue.pop_front();
                        self.inner = State::Idle;
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
                                VertexColorPacket::from_bits(
                                    psx.gpu.render_queue.pop_front().unwrap()
                                )
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

    /// Performs a VBlank.
    pub fn vblank(&mut self, psx: &mut PSX) {
        psx.gpu
            .status
            .set_interlace_odd(!psx.gpu.status.interlace_odd());
        psx.interrupts.status.request(Interrupt::VBlank);
        psx.scheduler
            .schedule(Event::VBlank, u64::from(psx.gpu.cycles_per_vblank()));

        self.renderer.exec(Command::VBlank);
    }
}
