pub mod interface;

mod display;
mod rendering;

use crate::{PSX, scheduler::Event};
use bitos::integer::{u9, u10, u11};
use interface::{Command, CopyToVram, Renderer, VramCoords, VramDimensions};
use shimmer_core::{
    gpu::{
        VerticalResolution,
        cmd::{
            DisplayCommand, RenderingCommand,
            rendering::{
                CoordPacket, LineCmd, ShadingMode, SizePacket, VertexColorPacket,
                VertexPositionPacket,
            },
        },
    },
    interrupts::Interrupt,
};
use tinylog::{debug, info, trace};

/// The state of the interpreter.
#[derive(Debug, Clone, Default)]
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
                    let effective_width = if size.width() == 0 {
                        0x400
                    } else {
                        ((size.width() - 1) & 0x3FF) + 1
                    };

                    let effective_height = if size.height() == 0 {
                        0x200
                    } else {
                        ((size.height() - 1) & 0x1FF) + 1
                    };

                    let count =
                        (u32::from(effective_width) * u32::from(effective_height)).div_ceil(2);
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
                        coords: VramCoords {
                            x: u10::new(dest.x()),
                            y: u9::new(dest.y()),
                        },
                        dimensions: VramDimensions {
                            width: u11::new(effective_width),
                            height: u10::new(effective_height),
                        },
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
        trace!(psx.loggers.gpu, "== VBLANK =="; state = self.inner.clone());
        if psx.gpu.status.vertical_resolution() == VerticalResolution::R480 {
            psx.gpu
                .status
                .set_interlace_odd(!psx.gpu.status.interlace_odd());
        } else {
            psx.gpu.status.set_interlace_odd(false);
        }

        psx.interrupts.status.request(Interrupt::VBlank);
        psx.scheduler
            .schedule(Event::VBlank, u64::from(psx.gpu.cycles_per_vblank()));

        self.renderer.exec(Command::VBlank);
    }
}
