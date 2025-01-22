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
use std::sync::mpsc::{Receiver, Sender};
use tinylog::debug;

#[derive(Debug, Default)]
enum Inner {
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
#[derive(Debug)]
pub struct Interpreter {
    inner: Inner,
    sender: Sender<Command>,
}

impl Interpreter {
    pub fn new() -> (Self, Receiver<Command>) {
        let (sender, receiver) = std::sync::mpsc::channel();
        (
            Self {
                inner: Inner::default(),
                sender,
            },
            receiver,
        )
    }

    fn exec_queued_render(&mut self, psx: &mut PSX) {
        match &mut self.inner {
            Inner::Idle => {
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
            Inner::CpuToVramBlit { dest: _dest, size } => {
                let count = (size.width() * size.height() + 1) / 2;
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

                self.sender
                    .send(Command::CopyToVram(CopyToVram {
                        x: u10::new(_dest.x()),
                        y: u10::new(_dest.y()),
                        width: u10::new(size.width()),
                        height: u10::new(size.height()),
                        data,
                    }))
                    .unwrap();

                self.inner = Inner::Idle;

                psx.gpu.status.set_ready_to_send_vram(false);
                psx.scheduler.schedule(Event::DmaUpdate, 0);

                self.exec_queued_render(psx);
            }
            Inner::PolyLine { cmd, received } => {
                let Some(front) = psx.gpu.render_queue.front() else {
                    return;
                };

                if *received >= 2 && (front & 0xF000_F000 == 0x5000_5000) {
                    debug!(psx.loggers.gpu, "exiting polyline mode",);
                    psx.gpu.render_queue.pop_front();
                    self.inner = Inner::Idle;
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

    /// Performs a VBlank.
    pub fn vblank(&mut self, psx: &mut PSX) {
        psx.gpu
            .status
            .set_interlace_odd(!psx.gpu.status.interlace_odd());
        self.sender.send(Command::VBlank).unwrap();

        psx.interrupts.status.request(Interrupt::VBlank);
        psx.scheduler
            .schedule(Event::VBlank, u64::from(psx.gpu.cycles_per_vblank()));
    }
}
