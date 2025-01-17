use crate::{
    PSX,
    gpu::{
        cmd::{
            EnvironmentOpcode, MiscOpcode, RenderingCommand, RenderingOpcode,
            environment::TexPage,
            rendering::{
                Clut, CoordPacket, LineMode, PolygonMode, RectangleMode, ShadingMode, SizePacket,
                VertexColorPacket, VertexPositionPacket, VertexUVPacket,
            },
        },
        interpreter::{Inner, Interpreter},
        renderer::{Action, Rgba8, UntexturedTriangle, Vertex},
    },
    scheduler::Event,
};
use bitos::integer::u1;
use tinylog::{debug, error, warn};

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
        let base_color = Rgba8::new(cmd.r(), cmd.g(), cmd.b());

        let mut clut = Clut::default();
        let mut texpage = TexPage::default();
        let mut vertex = |i| {
            let color = if i == 0 || cmd.shading_mode() == ShadingMode::Flat {
                base_color
            } else {
                let color = VertexColorPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());
                Rgba8::new(color.r(), color.g(), color.b())
            };

            let position =
                VertexPositionPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());

            let uv = if cmd.textured() {
                let uv = VertexUVPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());

                match i {
                    0 => {
                        clut = uv.clut();
                    }
                    1 => {
                        texpage = uv.texpage();

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
                    }
                    _ => (),
                }

                uv
            } else {
                VertexUVPacket::default()
            };

            Vertex {
                color,
                x: position.x(),
                y: position.y(),
                u: uv.u(),
                v: uv.v(),
                _padding: 0,
            }
        };

        let vertex_a = vertex(0);
        let vertex_b = vertex(1);
        let vertex_c = vertex(2);

        self.sender
            .send(Action::DrawUntexturedTriangle(UntexturedTriangle {
                vertices: [vertex_a, vertex_b, vertex_c],
            }))
            .unwrap();

        if cmd.polygon_mode() == PolygonMode::Rectangle {
            let vertex_d = vertex(3);
            self.sender
                .send(Action::DrawUntexturedTriangle(UntexturedTriangle {
                    vertices: [vertex_b, vertex_d, vertex_c],
                }))
                .unwrap();
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
