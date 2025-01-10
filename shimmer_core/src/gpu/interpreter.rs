use super::{
    ExecState,
    instr::{DisplayInstruction, Packet, RenderingInstruction},
};
use crate::{
    PSX,
    gpu::{
        DmaDirection, GpuStatus,
        instr::{
            DisplayOpcode, EnvironmentOpcode, MiscOpcode, RenderingOpcode,
            rendering::{CoordPacket, SizePacket, VertexPositionPacket},
        },
    },
};
use tinylog::{debug, trace};

pub struct Interpreter<'psx> {
    pub psx: &'psx mut PSX,
}

impl<'psx> Interpreter<'psx> {
    pub fn new(psx: &'psx mut PSX) -> Self {
        Self { psx }
    }

    /// Executes the given rendering instruction.
    pub fn exec_render(&mut self, instr: RenderingInstruction) {
        debug!(
            self.psx.loggers.gpu,
            "received render instr: {instr:?} (0x{:08X})",
            instr.into_bits()
        );

        match instr.opcode() {
            RenderingOpcode::Misc => match instr.misc_opcode().unwrap() {
                MiscOpcode::NOP => (),
                _ => debug!(self.psx.loggers.gpu, "unimplemented"),
            },
            RenderingOpcode::Environment => {
                let Some(opcode) = instr.environment_opcode() else {
                    return;
                };

                match opcode {
                    EnvironmentOpcode::DrawingSettings => {
                        let settings = instr.drawing_settings_instr();
                        let stat = &mut self.psx.gpu.status;

                        stat.set_texpage_x_base(settings.texpage_x_base());
                        stat.set_texpage_y_base(settings.texpage_y_base());
                        stat.set_semi_transparency_mode(settings.semi_transparency_mode());
                        stat.set_texpage_depth(settings.texpage_depth());
                        stat.set_compression_mode(settings.compression_mode());
                        stat.set_enable_drawing_to_display(settings.enable_drawing_to_display());
                        stat.set_texpage_y_base_2(settings.texpage_y_base_2());

                        self.psx.gpu.environment.textured_rect_flip_x =
                            settings.textured_rect_flip_x();
                        self.psx.gpu.environment.textured_rect_flip_y =
                            settings.textured_rect_flip_y();
                    }
                    _ => debug!(self.psx.loggers.gpu, "unimplemented"),
                }
            }
            RenderingOpcode::Polygon => {
                for _ in 0..instr.args() {
                    debug!(
                        self.psx.loggers.gpu,
                        "vertex: {:?}",
                        VertexPositionPacket::from_bits(
                            self.psx.gpu.queue.pop_front().unwrap().value()
                        )
                    );
                }

                return;
            }
            RenderingOpcode::CpuToVramBlit => {
                let dest = CoordPacket::from_bits(self.psx.gpu.queue.pop_front().unwrap().value());
                let size = SizePacket::from_bits(self.psx.gpu.queue.pop_front().unwrap().value());
                self.psx.gpu.execution_state = ExecState::CpuToVramBlit { dest, size };

                return;
            }
            _ => debug!(self.psx.loggers.gpu, "unimplemented"),
        }

        for _ in 0..instr.args() {
            debug!(
                self.psx.loggers.gpu,
                "instr arg: {:?}",
                self.psx.gpu.queue.pop_front()
            );
        }
    }

    /// Executes the given display instruction.
    pub fn exec_display(&mut self, instr: DisplayInstruction) {
        debug!(self.psx.loggers.gpu, "received display instr: {instr:?}");

        match instr.opcode().unwrap() {
            DisplayOpcode::ResetGpu => {
                self.psx.gpu.status = GpuStatus::default();
            }
            DisplayOpcode::DisplayMode => {
                let settings = instr.display_mode_instr();
                let stat = &mut self.psx.gpu.status;

                stat.set_horizontal_resolution(settings.horizontal_resolution());
                stat.set_vertical_resolution(settings.vertical_resolution());
                stat.set_video_mode(settings.video_mode());
                stat.set_display_depth(settings.display_depth());
                stat.set_vertical_interlace(settings.vertical_interlace());
                stat.set_force_horizontal_368(settings.force_horizontal_368());
                stat.set_flip_screen_x(settings.flip_screen_x());
            }
            DisplayOpcode::DmaDirection => {
                let instr = instr.dma_direction_instr();
                let dir = instr.direction();
                self.psx.gpu.status.set_dma_direction(dir);

                match dir {
                    DmaDirection::Off => self.psx.gpu.status.set_dma_request(false),
                    // TODO: fifo state
                    DmaDirection::Fifo => self.psx.gpu.status.set_dma_request(true),
                    DmaDirection::CpuToGp0 => self
                        .psx
                        .gpu
                        .status
                        .set_dma_request(self.psx.gpu.status.ready_to_receive_block()),
                    DmaDirection::GpuToCpu => self
                        .psx
                        .gpu
                        .status
                        .set_dma_request(self.psx.gpu.status.ready_to_send_vram()),
                };
            }
            DisplayOpcode::DisplayArea => {
                let settings = instr.display_area_instr();
                self.psx.gpu.display.area_start_x = settings.x();
                self.psx.gpu.display.area_start_y = settings.y();
            }
            DisplayOpcode::HorizontalDisplayRange => {
                let settings = instr.horizontal_display_range_instr();
                self.psx.gpu.display.horizontal_range = settings.x1()..settings.x2();
            }
            DisplayOpcode::VerticalDisplayRange => {
                let settings = instr.vertical_dispaly_range_instr();
                self.psx.gpu.display.vertical_range = settings.y1()..settings.y2();
            }
            _ => debug!(self.psx.loggers.gpu, "unimplemented"),
        }
    }

    /// Executes all queued GPU instructions.
    pub fn exec_queued(&mut self) {
        while !self.psx.gpu.queue.is_empty() {
            match &self.psx.gpu.execution_state {
                ExecState::None => {
                    let instr = self.psx.gpu.queue.front().unwrap();
                    let args = match instr {
                        Packet::Rendering(packet) => {
                            RenderingInstruction::from_bits(*packet).args()
                        }
                        Packet::Display(_) => 0,
                    };

                    if self.psx.gpu.queue.len() <= args {
                        break;
                    }

                    let instr = self.psx.gpu.queue.pop_front().unwrap();
                    match instr {
                        Packet::Rendering(packet) => {
                            self.exec_render(RenderingInstruction::from_bits(packet))
                        }
                        Packet::Display(packet) => {
                            self.exec_display(DisplayInstruction::from_bits(packet))
                        }
                    }
                }
                ExecState::CpuToVramBlit { dest, size } => {
                    let packets = (size.width() * size.height() + 1) / 2;
                    trace!(self.psx.loggers.gpu, "packet count: {:#08X}", packets);

                    if self.psx.gpu.queue.len() <= packets as usize {
                        break;
                    }

                    for _ in 0..packets {
                        let packet = self.psx.gpu.queue.pop_front().unwrap();
                        trace!(
                            self.psx.loggers.gpu,
                            "cpu to vram packet: {:#08X}",
                            packet.value()
                        );
                    }

                    self.psx.gpu.execution_state = ExecState::None;
                }
            }
        }
    }
}
