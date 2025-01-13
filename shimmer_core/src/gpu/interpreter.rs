use super::{
    ExecState,
    cmd::{DisplayCommand, Packet, RenderingCommand},
};
use crate::{
    PSX,
    gpu::{
        DmaDirection, GpuStatus,
        cmd::{
            DisplayOpcode, EnvironmentOpcode, MiscOpcode, RenderingOpcode,
            rendering::{
                CoordPacket, ShadingMode, SizePacket, VertexColorPacket, VertexPositionPacket,
                VertexUVPacket,
            },
        },
    },
};
use tinylog::{debug, error, warn};

#[derive(Default)]
pub struct Interpreter {
    // no state currently
}

impl Interpreter {
    fn update_dma_request(&mut self, psx: &mut PSX) {
        let dir = psx.gpu.status.dma_direction();
        match dir {
            DmaDirection::Off => psx.gpu.status.set_dma_request(true),
            DmaDirection::Fifo => psx.gpu.status.set_dma_request(true),
            DmaDirection::CpuToGp0 => psx
                .gpu
                .status
                .set_dma_request(psx.gpu.status.ready_to_receive_block()),
            DmaDirection::GpuToCpu => psx
                .gpu
                .status
                .set_dma_request(psx.gpu.status.ready_to_send_vram()),
        };
    }

    /// Executes the given rendering command.
    pub fn exec_render(&mut self, psx: &mut PSX, cmd: RenderingCommand) {
        debug!(
            psx.loggers.gpu,
            "received render cmd: {cmd:?} (0x{:08X})",
            cmd.into_bits()
        );

        match cmd.opcode() {
            RenderingOpcode::Misc => {
                match cmd.misc_opcode().unwrap() {
                    MiscOpcode::NOP => (),
                    MiscOpcode::ClearCache => (),
                    _ => warn!(psx.loggers.gpu, "unimplemented rendering (misc) command"),
                }

                psx.gpu.status.set_ready_to_receive_cmd(true);
            }
            RenderingOpcode::Environment => {
                match cmd.environment_opcode().unwrap() {
                    EnvironmentOpcode::DrawingSettings => {
                        let settings = cmd.drawing_settings_cmd();
                        let stat = &mut psx.gpu.status;

                        stat.set_texpage_x_base(settings.texpage_x_base());
                        stat.set_texpage_y_base(settings.texpage_y_base());
                        stat.set_semi_transparency_mode(settings.semi_transparency_mode());
                        stat.set_texpage_depth(settings.texpage_depth());
                        stat.set_compression_mode(settings.compression_mode());
                        stat.set_enable_drawing_to_display(settings.enable_drawing_to_display());
                        stat.set_texpage_y_base_2(settings.texpage_y_base_2());

                        psx.gpu.environment.textured_rect_flip_x = settings.textured_rect_flip_x();
                        psx.gpu.environment.textured_rect_flip_y = settings.textured_rect_flip_y();
                    }
                    _ => warn!(
                        psx.loggers.gpu,
                        "unimplemented rendering (environment) command"
                    ),
                }

                psx.gpu.status.set_ready_to_receive_cmd(true);
            }
            RenderingOpcode::Polygon => {
                let cmd = cmd.polygon_cmd();
                for _ in 0..cmd.polygon_mode().vertices() {
                    if cmd.shading_mode() == ShadingMode::Gouraud {
                        debug!(
                            psx.loggers.gpu,
                            "gouraud: {:?}",
                            VertexColorPacket::from_bits(psx.gpu.queue.pop_render().unwrap())
                        );
                    }

                    debug!(
                        psx.loggers.gpu,
                        "vertex: {:?}",
                        VertexPositionPacket::from_bits(psx.gpu.queue.pop_render().unwrap())
                    );

                    if cmd.textured() {
                        debug!(
                            psx.loggers.gpu,
                            "vertex: {:?}",
                            VertexUVPacket::from_bits(psx.gpu.queue.pop_render().unwrap())
                        );
                    }
                }

                psx.gpu.status.set_ready_to_receive_cmd(true);
            }
            RenderingOpcode::CpuToVramBlit => {
                let dest = CoordPacket::from_bits(psx.gpu.queue.pop_render().unwrap());
                let size = SizePacket::from_bits(psx.gpu.queue.pop_render().unwrap());
                psx.gpu.execution_state = ExecState::CpuToVramBlit { dest, size };
            }
            RenderingOpcode::VramToCpuBlit => {
                debug!(psx.loggers.gpu, "coord: {:?}", psx.gpu.queue.pop_render());
                debug!(
                    psx.loggers.gpu,
                    "dimensions: {:?}",
                    psx.gpu.queue.pop_render()
                );

                psx.gpu.status.set_ready_to_send_vram(true);

                // TODO: actually send
                psx.gpu.status.set_ready_to_receive_cmd(true);
            }
            _ => error!(psx.loggers.gpu, "unimplemented rendering command"),
        }
    }

    /// Executes the given display command.
    pub fn exec_display(&mut self, psx: &mut PSX, cmd: DisplayCommand) {
        debug!(psx.loggers.gpu, "received display cmd: {cmd:?}");

        match cmd.opcode().unwrap() {
            DisplayOpcode::ResetGpu => {
                // TODO: reset internal registers
                psx.gpu.status = GpuStatus::default();
            }
            DisplayOpcode::DisplayMode => {
                let settings = cmd.display_mode_cmd();
                let stat = &mut psx.gpu.status;

                stat.set_horizontal_resolution(settings.horizontal_resolution());
                stat.set_vertical_resolution(settings.vertical_resolution());
                stat.set_video_mode(settings.video_mode());
                stat.set_display_depth(settings.display_depth());
                stat.set_vertical_interlace(settings.vertical_interlace());
                stat.set_force_horizontal_368(settings.force_horizontal_368());
                stat.set_flip_screen_x(settings.flip_screen_x());
            }
            DisplayOpcode::DmaDirection => {
                let cmd = cmd.dma_direction_cmd();
                psx.gpu.status.set_dma_direction(cmd.direction());

                self.update_dma_request(psx);
            }
            DisplayOpcode::DisplayArea => {
                let settings = cmd.display_area_cmd();
                psx.gpu.display.area_start_x = settings.x();
                psx.gpu.display.area_start_y = settings.y();
            }
            DisplayOpcode::HorizontalDisplayRange => {
                let settings = cmd.horizontal_display_range_cmd();
                psx.gpu.display.horizontal_range = settings.x1()..settings.x2();
            }
            DisplayOpcode::VerticalDisplayRange => {
                let settings = cmd.vertical_dispaly_range_cmd();
                psx.gpu.display.vertical_range = settings.y1()..settings.y2();
            }
            DisplayOpcode::DisplayEnabled => {
                let settings = cmd.display_enable_cmd();
                psx.gpu.status.set_disable_display(settings.disabled());
            }
            _ => warn!(psx.loggers.gpu, "unimplemented display command"),
        }
    }

    /// Executes all queued GPU commands.
    pub fn exec_queued(&mut self, psx: &mut PSX) {
        self.update_dma_request(psx);

        psx.gpu.status.set_ready_to_receive_cmd(true);
        while !psx.gpu.queue.is_empty() {
            match &psx.gpu.execution_state {
                ExecState::None => {
                    let cmd = psx.gpu.queue.front().unwrap();
                    match cmd {
                        Packet::Rendering(packet) => {
                            let cmd = RenderingCommand::from_bits(*packet);
                            if psx.gpu.queue.render_len() <= cmd.args() {
                                break;
                            }

                            psx.gpu.queue.pop();
                            self.exec_render(psx, cmd);
                        }
                        Packet::Display(packet) => {
                            let cmd = DisplayCommand::from_bits(*packet);

                            psx.gpu.queue.pop();
                            self.exec_display(psx, cmd);
                        }
                    };
                }
                ExecState::CpuToVramBlit { dest: _, size } => {
                    let packets = (size.width() * size.height() + 1) / 2;
                    if psx.gpu.queue.render_len() <= packets as usize {
                        break;
                    }

                    for _ in 0..packets {
                        // TODO: perform blit
                        let _packet = psx.gpu.queue.pop_render().unwrap();
                    }

                    psx.gpu.status.set_ready_to_receive_cmd(true);
                    psx.gpu.execution_state = ExecState::None;
                }
            }
        }
    }
}
