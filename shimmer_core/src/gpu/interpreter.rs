use super::{
    ExecState,
    cmd::{DisplayCommand, Packet, RenderingCommand},
};
use crate::{
    PSX,
    gpu::{
        GpuStatus,
        cmd::{
            DisplayOpcode, EnvironmentOpcode, MiscOpcode, RenderingOpcode,
            rendering::{
                CoordPacket, RectangleMode, ShadingMode, SizePacket, VertexColorPacket,
                VertexPositionPacket, VertexUVPacket,
            },
        },
    },
    scheduler::Event,
};
use bitos::integer::u1;
use tinylog::{debug, error};

#[derive(Default)]
pub struct Interpreter {
    // no state currently
}

impl Interpreter {
    /// Executes the given rendering command.
    pub fn exec_render(&mut self, psx: &mut PSX, cmd: RenderingCommand) {
        debug!(
            psx.loggers.gpu,
            "received render cmd: {cmd:?} (0x{:08X})",
            cmd.into_bits()
        );

        match cmd.opcode() {
            RenderingOpcode::Misc => match cmd.misc_opcode().unwrap() {
                MiscOpcode::NOP => (),
                MiscOpcode::ClearCache => (),
                MiscOpcode::QuickRectangleFill => {
                    debug!(
                        psx.loggers.gpu,
                        "top left: {:?}",
                        CoordPacket::from_bits(psx.gpu.queue.pop_render().unwrap())
                    );

                    debug!(
                        psx.loggers.gpu,
                        "dimensions: {:?}",
                        SizePacket::from_bits(psx.gpu.queue.pop_render().unwrap())
                    );
                }
                _ => error!(
                    psx.loggers.gpu,
                    "unimplemented rendering (misc) command: {:?}",
                    cmd.misc_opcode()
                ),
            },
            RenderingOpcode::Environment => match cmd.environment_opcode().unwrap() {
                EnvironmentOpcode::DrawingSettings => {
                    let settings = cmd.drawing_settings_cmd();
                    let stat = &mut psx.gpu.status;

                    stat.set_texpage_x_base(settings.texpage().texpage_x_base());
                    stat.set_texpage_y_base(settings.texpage().texpage_y_base());
                    stat.set_semi_transparency_mode(settings.texpage().semi_transparency_mode());
                    stat.set_texpage_depth(settings.texpage().texpage_depth());
                    stat.set_compression_mode(settings.compression_mode());
                    stat.set_enable_drawing_to_display(settings.enable_drawing_to_display());

                    if psx.gpu.environment.double_vram {
                        stat.set_texpage_y_base_2(settings.texpage_y_base_2());
                    } else {
                        stat.set_texpage_y_base_2(u1::new(0));
                    }

                    psx.gpu.environment.textured_rect_flip_x = settings.textured_rect_flip_x();
                    psx.gpu.environment.textured_rect_flip_y = settings.textured_rect_flip_y();
                }
                _ => error!(
                    psx.loggers.gpu,
                    "unimplemented rendering (environment) command: {:?}",
                    cmd.environment_opcode()
                ),
            },
            RenderingOpcode::Polygon => {
                let cmd = cmd.polygon_cmd();
                for index in 0..cmd.polygon_mode().vertices() {
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
                        let uv = VertexUVPacket::from_bits(psx.gpu.queue.pop_render().unwrap());
                        debug!(psx.loggers.gpu, "uv: {:?}", uv.clone());

                        if index == 1 {
                            let stat = &mut psx.gpu.status;
                            stat.set_texpage_x_base(uv.texpage().texpage_x_base());
                            stat.set_texpage_y_base(uv.texpage().texpage_y_base());
                            stat.set_semi_transparency_mode(uv.texpage().semi_transparency_mode());
                            stat.set_texpage_depth(uv.texpage().texpage_depth());

                            if psx.gpu.environment.double_vram {
                                stat.set_texpage_y_base_2(uv.texpage().texpage_y_base_2());
                            }
                        }
                    }
                }
            }
            RenderingOpcode::CpuToVramBlit => {
                let dest = CoordPacket::from_bits(psx.gpu.queue.pop_render().unwrap());
                let size = SizePacket::from_bits(psx.gpu.queue.pop_render().unwrap());
                psx.gpu.execution_state = ExecState::CpuToVramBlit { dest, size };

                psx.gpu.status.set_ready_to_send_vram(true);
                psx.scheduler.schedule(Event::DmaUpdate, 0);
            }
            RenderingOpcode::VramToCpuBlit => {
                debug!(psx.loggers.gpu, "coord: {:?}", psx.gpu.queue.pop_render());
                debug!(
                    psx.loggers.gpu,
                    "dimensions: {:?}",
                    psx.gpu.queue.pop_render()
                );

                psx.gpu.status.set_ready_to_send_vram(true);
                psx.scheduler.schedule(Event::DmaUpdate, 0);
            }
            RenderingOpcode::Rectangle => {
                let cmd = cmd.rectangle_cmd();

                debug!(
                    psx.loggers.gpu,
                    "top left: {:?}",
                    VertexPositionPacket::from_bits(psx.gpu.queue.pop_render().unwrap())
                );

                if cmd.textured() {
                    debug!(
                        psx.loggers.gpu,
                        "uv: {:?}",
                        VertexUVPacket::from_bits(psx.gpu.queue.pop_render().unwrap())
                    );
                }

                if cmd.rectangle_mode() == RectangleMode::Variable {
                    debug!(
                        psx.loggers.gpu,
                        "size: {:?}",
                        SizePacket::from_bits(psx.gpu.queue.pop_render().unwrap())
                    );
                }
            }
            _ => error!(psx.loggers.gpu, "unimplemented rendering command: {cmd:?}"),
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
                psx.scheduler.schedule(Event::DmaUpdate, 0);
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
            DisplayOpcode::VramSizeV2 => {
                let settings = cmd.vram_size_cmd();
                psx.gpu.environment.double_vram = settings.double();
            }
            _ => error!(psx.loggers.gpu, "unimplemented display command: {cmd:?}"),
        }
    }

    /// Executes all queued GPU commands.
    pub fn exec_queued(&mut self, psx: &mut PSX) {
        while !psx.gpu.queue.is_empty() {
            match &psx.gpu.execution_state {
                ExecState::None => {
                    let cmd = psx.gpu.queue.front().unwrap();
                    match cmd {
                        Packet::Rendering(packet) => {
                            let cmd = RenderingCommand::from_bits(*packet);
                            if psx.gpu.queue.render_len() <= cmd.args() {
                                debug!(
                                    psx.loggers.gpu,
                                    "{cmd:?} is waiting for > {} arguments (has {})",
                                    cmd.args(),
                                    psx.gpu.queue.render_len(),
                                );
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

                    psx.gpu.execution_state = ExecState::None;

                    psx.gpu.status.set_ready_to_send_vram(false);
                    psx.scheduler.schedule(Event::DmaUpdate, 0);
                }
            }
        }
    }
}
