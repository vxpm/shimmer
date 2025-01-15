use crate::{
    PSX,
    gpu::{
        cmd::{
            EnvironmentOpcode, MiscOpcode, RenderingCommand, RenderingOpcode,
            rendering::{
                CoordPacket, LineMode, RectangleMode, ShadingMode, SizePacket, VertexColorPacket,
                VertexPositionPacket, VertexUVPacket,
            },
        },
        interpreter::{Inner, Interpreter},
    },
    scheduler::Event,
};
use bitos::integer::u1;
use tinylog::{debug, error};

impl Interpreter {
    /// Executes the given rendering command.
    pub fn exec_render(&mut self, psx: &mut PSX, cmd: RenderingCommand) {
        debug!(
            psx.loggers.gpu,
            "received render cmd: {cmd:?} (0x{:08X})",
            cmd.to_bits()
        );

        match cmd.opcode() {
            RenderingOpcode::Misc => match cmd.misc_opcode().unwrap() {
                MiscOpcode::NOP | MiscOpcode::ClearCache => (),
                MiscOpcode::QuickRectangleFill => {
                    debug!(
                        psx.loggers.gpu,
                        "top left: {:?}",
                        CoordPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap())
                    );

                    debug!(
                        psx.loggers.gpu,
                        "dimensions: {:?}",
                        SizePacket::from_bits(psx.gpu.render_queue.pop_front().unwrap())
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

                    stat.set_texpage_x_base(settings.texpage().x_base());
                    stat.set_texpage_y_base(settings.texpage().y_base());
                    stat.set_semi_transparency_mode(settings.texpage().semi_transparency_mode());
                    stat.set_texpage_depth(settings.texpage().depth());
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
                    if index != 0 && cmd.shading_mode() == ShadingMode::Gouraud {
                        debug!(
                            psx.loggers.gpu,
                            "gouraud: {:?}",
                            VertexColorPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap())
                        );
                    }

                    debug!(
                        psx.loggers.gpu,
                        "vertex: {:?}",
                        VertexPositionPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap())
                    );

                    if cmd.textured() {
                        let uv =
                            VertexUVPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());

                        debug!(psx.loggers.gpu, "u: {:?} v: {:?}", uv.u(), uv.v());

                        if index == 0 {
                            debug!(psx.loggers.gpu, "clut: {:?}", uv.clut());
                        } else if index == 1 {
                            debug!(psx.loggers.gpu, "page: {:?} ", uv.texpage());

                            let stat = &mut psx.gpu.status;
                            stat.set_texpage_x_base(uv.texpage().x_base());
                            stat.set_texpage_y_base(uv.texpage().y_base());
                            stat.set_semi_transparency_mode(uv.texpage().semi_transparency_mode());
                            stat.set_texpage_depth(uv.texpage().depth());

                            if psx.gpu.environment.double_vram {
                                stat.set_texpage_y_base_2(uv.texpage().y_base_2());
                            } else {
                                stat.set_texpage_y_base_2(u1::new(0));
                            }
                        }
                    }
                }
            }
            RenderingOpcode::CpuToVramBlit => {
                let dest = CoordPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());
                let size = SizePacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());

                debug!(psx.loggers.gpu, "starting CPU to VRAM blit"; dest = dest.clone(), size = size.clone());
                self.inner = Inner::CpuToVramBlit { _dest: dest, size };

                psx.gpu.status.set_ready_to_send_vram(true);
                psx.scheduler.schedule(Event::DmaUpdate, 0);
            }
            RenderingOpcode::VramToCpuBlit => {
                // for now, nop
                psx.gpu.status.set_ready_to_send_vram(true);

                // let src = CoordPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());
                // let size = SizePacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());
                //
                // psx.gpu.status.set_ready_to_send_vram(true);
                // psx.scheduler.schedule(Event::DmaUpdate, 0);
            }
            RenderingOpcode::Rectangle => {
                let cmd = cmd.rectangle_cmd();

                debug!(
                    psx.loggers.gpu,
                    "top left: {:?}",
                    VertexPositionPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap())
                );

                if cmd.textured() {
                    debug!(
                        psx.loggers.gpu,
                        "uv: {:?}",
                        VertexUVPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap())
                    );
                }

                if cmd.rectangle_mode() == RectangleMode::Variable {
                    debug!(
                        psx.loggers.gpu,
                        "size: {:?}",
                        SizePacket::from_bits(psx.gpu.render_queue.pop_front().unwrap())
                    );
                }
            }
            RenderingOpcode::Line => {
                let cmd = cmd.line_cmd();

                match cmd.line_mode() {
                    LineMode::Single => {
                        for _ in 0..2 {
                            if cmd.shading_mode() == ShadingMode::Gouraud {
                                debug!(
                                    psx.loggers.gpu,
                                    "gouraud: {:?}",
                                    VertexColorPacket::from_bits(
                                        psx.gpu.render_queue.pop_front().unwrap()
                                    )
                                );
                            }

                            debug!(
                                psx.loggers.gpu,
                                "vertex: {:?}",
                                VertexPositionPacket::from_bits(
                                    psx.gpu.render_queue.pop_front().unwrap()
                                )
                            );
                        }
                    }
                    LineMode::Poly => {
                        debug!(psx.loggers.gpu, "starting polyline mode",);
                        self.inner = Inner::PolyLine { cmd, received: 0 };
                    }
                }
            }
            _ => error!(psx.loggers.gpu, "unimplemented rendering command: {cmd:?}"),
        }
    }
}
