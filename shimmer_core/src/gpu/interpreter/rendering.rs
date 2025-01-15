use crate::{
    PSX,
    gpu::{
        cmd::{
            EnvironmentOpcode, MiscOpcode, RenderingCommand, RenderingOpcode,
            rendering::{
                CoordPacket, LineMode, PolygonMode, RectangleMode, ShadingMode, SizePacket,
                VertexColorPacket, VertexPositionPacket, VertexUVPacket,
            },
        },
        interpreter::{Inner, Interpreter},
        renderer::{Action, Rgba, UntexturedTriangle, Vertex},
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
                let textured_color = Rgba {
                    r: 0,
                    g: 0,
                    b: 180,
                    a: 255,
                };
                let base_color = Rgba {
                    r: cmd.color_r(),
                    g: cmd.color_g(),
                    b: cmd.color_b(),
                    a: 255,
                };

                let vertex_a_xy =
                    VertexPositionPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());

                let vertex_a_uv = if cmd.textured() {
                    VertexUVPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap())
                } else {
                    VertexUVPacket::from_bits(0)
                };

                let vertex_a = Vertex {
                    color: if cmd.textured() {
                        textured_color
                    } else {
                        base_color
                    },
                    x: vertex_a_xy.x(),
                    y: vertex_a_xy.y(),
                    u: vertex_a_uv.u(),
                    v: vertex_a_uv.v(),
                    padding: 0,
                };

                let vertex_b_rgba = if cmd.shading_mode() == ShadingMode::Gouraud {
                    let color =
                        VertexColorPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());

                    Rgba {
                        r: color.color_r(),
                        g: color.color_g(),
                        b: color.color_b(),
                        a: 255,
                    }
                } else {
                    base_color
                };

                let vertex_b_xy =
                    VertexPositionPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());

                let vertex_b_uv = if cmd.textured() {
                    let uv = VertexUVPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());

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

                    uv
                } else {
                    VertexUVPacket::from_bits(0)
                };

                let vertex_b = Vertex {
                    color: if cmd.textured() {
                        textured_color
                    } else {
                        vertex_b_rgba
                    },
                    x: vertex_b_xy.x(),
                    y: vertex_b_xy.y(),
                    u: vertex_b_uv.u(),
                    v: vertex_b_uv.v(),
                    padding: 0,
                };

                let vertex_c_rgba = if cmd.shading_mode() == ShadingMode::Gouraud {
                    let color =
                        VertexColorPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());

                    Rgba {
                        r: color.color_r(),
                        g: color.color_g(),
                        b: color.color_b(),
                        a: 255,
                    }
                } else {
                    base_color
                };

                let vertex_c_xy =
                    VertexPositionPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());

                let vertex_c_uv = if cmd.textured() {
                    VertexUVPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap())
                } else {
                    VertexUVPacket::from_bits(0)
                };

                let vertex_c = Vertex {
                    color: if cmd.textured() {
                        textured_color
                    } else {
                        vertex_c_rgba
                    },
                    x: vertex_c_xy.x(),
                    y: vertex_c_xy.y(),
                    u: vertex_c_uv.u(),
                    v: vertex_c_uv.v(),
                    padding: 0,
                };

                self.sender
                    .send(Action::DrawUntexturedTriangle(UntexturedTriangle {
                        vertices: [vertex_a, vertex_b, vertex_c],
                        shading_mode: ShadingMode::Flat,
                    }))
                    .unwrap();

                if cmd.polygon_mode() == PolygonMode::Rectangle {
                    let vertex_d_rgba = if cmd.shading_mode() == ShadingMode::Gouraud {
                        let color =
                            VertexColorPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());

                        Rgba {
                            r: color.color_r(),
                            g: color.color_g(),
                            b: color.color_b(),
                            a: 255,
                        }
                    } else {
                        base_color
                    };

                    let vertex_d_xy =
                        VertexPositionPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());

                    let vertex_d_uv = if cmd.textured() {
                        VertexUVPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap())
                    } else {
                        VertexUVPacket::from_bits(0)
                    };

                    let vertex_d = Vertex {
                        color: if cmd.textured() {
                            textured_color
                        } else {
                            vertex_d_rgba
                        },
                        x: vertex_d_xy.x(),
                        y: vertex_d_xy.y(),
                        u: vertex_d_uv.u(),
                        v: vertex_d_uv.v(),
                        padding: 0,
                    };

                    self.sender
                        .send(Action::DrawUntexturedTriangle(UntexturedTriangle {
                            vertices: [vertex_b, vertex_d, vertex_c],
                            shading_mode: ShadingMode::Flat,
                        }))
                        .unwrap();
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
