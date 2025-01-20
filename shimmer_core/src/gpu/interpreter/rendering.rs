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
        renderer::{Action, Rgba8, TexturedTriangle, UntexturedTriangle, Vertex},
    },
    scheduler::Event,
};
use bitos::integer::u1;
use tinylog::{debug, error, warn};

#[derive(Default)]
struct VertexPackets {
    color: VertexColorPacket,
    position: VertexPositionPacket,
    uv: VertexUVPacket,
}

impl VertexPackets {
    fn to_vertex(&self) -> Vertex {
        Vertex {
            color: Rgba8::new(self.color.r(), self.color.g(), self.color.b()),
            x: self.position.x(),
            y: self.position.y(),
            u: self.uv.u(),
            v: self.uv.v(),
            _padding: 0,
        }
    }
}

impl Interpreter {
    fn exec_quick_rect_fill(&mut self, psx: &mut PSX, _: RenderingCommand) {
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

    fn exec_polygon(&mut self, psx: &mut PSX, cmd: RenderingCommand) {
        let cmd = cmd.polygon_cmd();
        let base_color_packet = VertexColorPacket::default()
            .with_r(cmd.r())
            .with_g(cmd.g())
            .with_b(cmd.b());

        let mut vertex = |skip_color| {
            let color = if skip_color || cmd.shading_mode() == ShadingMode::Flat {
                base_color_packet
            } else {
                VertexColorPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap())
            };

            let position =
                VertexPositionPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());

            let uv = if cmd.textured() {
                VertexUVPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap())
            } else {
                VertexUVPacket::default()
            };

            VertexPackets {
                color,
                position,
                uv,
            }
        };

        let vertex_a = vertex(true);
        let vertex_b = vertex(false);
        let vertex_c = vertex(false);
        let vertex_d = if cmd.polygon_mode() == PolygonMode::Rectangle {
            vertex(false)
        } else {
            VertexPackets::default()
        };

        let clut = vertex_a.uv.clut();
        let texpage = vertex_b.uv.texpage();

        let tri_1 = [
            vertex_a.to_vertex(),
            vertex_b.to_vertex(),
            vertex_c.to_vertex(),
        ];
        let tri_2 = [
            vertex_b.to_vertex(),
            vertex_d.to_vertex(),
            vertex_c.to_vertex(),
        ];

        if cmd.textured() {
            let stat = &mut psx.gpu.status;
            stat.set_texpage_x_base(texpage.x_base());
            stat.set_texpage_y_base(texpage.y_base());
            stat.set_semi_transparency_mode(texpage.semi_transparency_mode());
            stat.set_texpage_depth(texpage.depth());

            if psx.gpu.environment.double_vram {
                stat.set_texpage_y_base_2(texpage.y_base_2());
            } else {
                stat.set_texpage_y_base_2(u1::new(0));
            }

            self.sender
                .send(Action::DrawTexturedTriangle(TexturedTriangle {
                    vertices: tri_1,
                    shading: cmd.shading_mode(),
                    clut,
                    texpage,
                }))
                .unwrap();

            if cmd.polygon_mode() == PolygonMode::Rectangle {
                self.sender
                    .send(Action::DrawTexturedTriangle(TexturedTriangle {
                        vertices: tri_2,
                        shading: cmd.shading_mode(),
                        clut,
                        texpage,
                    }))
                    .unwrap();
            }
        } else {
            self.sender
                .send(Action::DrawUntexturedTriangle(UntexturedTriangle {
                    vertices: tri_1,
                    shading: cmd.shading_mode(),
                }))
                .unwrap();

            if cmd.polygon_mode() == PolygonMode::Rectangle {
                self.sender
                    .send(Action::DrawUntexturedTriangle(UntexturedTriangle {
                        vertices: tri_2,
                        shading: cmd.shading_mode(),
                    }))
                    .unwrap();
            }
        }
    }

    fn exec_drawing_settings(&mut self, psx: &mut PSX, cmd: RenderingCommand) {
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

    fn exec_cpu_to_vram_blit(&mut self, psx: &mut PSX, _: RenderingCommand) {
        let dest = CoordPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());
        let size = SizePacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());

        debug!(psx.loggers.gpu, "starting CPU to VRAM blit"; dest = dest.clone(), size = size.clone());
        self.inner = Inner::CpuToVramBlit { dest, size };

        psx.gpu.status.set_ready_to_send_vram(true);
        psx.scheduler.schedule(Event::DmaUpdate, 0);
    }

    fn exec_vram_to_cpu_blit(&mut self, psx: &mut PSX, _: RenderingCommand) {
        // for now, nop
        psx.gpu.status.set_ready_to_send_vram(true);

        // let src = CoordPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());
        // let size = SizePacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());
        //
        // psx.gpu.status.set_ready_to_send_vram(true);
        // psx.scheduler.schedule(Event::DmaUpdate, 0);
    }

    fn exec_rectangle(&mut self, psx: &mut PSX, cmd: RenderingCommand) {
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

    fn exec_line(&mut self, psx: &mut PSX, cmd: RenderingCommand) {
        let cmd = cmd.line_cmd();
        match cmd.line_mode() {
            LineMode::Single => {
                for _ in 0..2 {
                    if cmd.shading_mode() == ShadingMode::Gouraud {
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
                }
            }
            LineMode::Poly => {
                debug!(psx.loggers.gpu, "starting polyline mode",);
                self.inner = Inner::PolyLine { cmd, received: 0 };
            }
        }
    }

    /// Executes the given rendering command.
    pub fn exec_render(&mut self, psx: &mut PSX, cmd: RenderingCommand) {
        debug!(
            psx.loggers.gpu,
            "executing render cmd: {cmd:?} (0x{:08X})",
            cmd.to_bits()
        );

        match cmd.opcode() {
            RenderingOpcode::Misc => match cmd.misc_opcode().unwrap() {
                MiscOpcode::NOP => (),
                MiscOpcode::ClearCache => warn!(psx.loggers.gpu, "should have cleared cache"),
                MiscOpcode::QuickRectangleFill => self.exec_quick_rect_fill(psx, cmd),
                _ => error!(
                    psx.loggers.gpu,
                    "unimplemented rendering (misc) command: {:?}",
                    cmd.misc_opcode()
                ),
            },
            RenderingOpcode::Environment => match cmd.environment_opcode().unwrap() {
                EnvironmentOpcode::DrawingSettings => self.exec_drawing_settings(psx, cmd),
                _ => error!(
                    psx.loggers.gpu,
                    "unimplemented rendering (environment) command: {:?}",
                    cmd.environment_opcode()
                ),
            },
            RenderingOpcode::Polygon => self.exec_polygon(psx, cmd),
            RenderingOpcode::CpuToVramBlit => self.exec_cpu_to_vram_blit(psx, cmd),
            RenderingOpcode::VramToCpuBlit => self.exec_vram_to_cpu_blit(psx, cmd),
            RenderingOpcode::Rectangle => self.exec_rectangle(psx, cmd),
            RenderingOpcode::Line => self.exec_line(psx, cmd),
            _ => error!(psx.loggers.gpu, "unimplemented rendering command: {cmd:?}"),
        }
    }
}
