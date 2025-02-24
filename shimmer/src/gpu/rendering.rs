use super::Gpu;
use crate::{
    PSX,
    gpu::{
        State,
        interface::{
            Command, CopyFromVram, DrawingArea, DrawingSettings, Rgba8, TexConfig, VramCoords,
            VramDimensions,
            primitive::{Primitive, Rectangle, Triangle, Vertex},
        },
    },
    scheduler::Event,
};
use bitos::integer::{i11, u9, u10, u11};
use shimmer_core::gpu::cmd::{
    EnvironmentOpcode, MiscOpcode, RenderingCommand, RenderingOpcode,
    rendering::{
        CoordPacket, LineMode, PolygonMode, RectangleMode, ShadingMode, SizePacket,
        TransparencyMode, VertexColorPacket, VertexPositionPacket, VertexUVPacket,
    },
};
use tinylog::{debug, error, info, trace};

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
        }
    }
}

impl Gpu {
    fn exec_quick_rect_fill(&mut self, psx: &mut PSX, cmd: RenderingCommand) {
        let cmd = cmd.rectangle_cmd();
        let color = Rgba8::new(cmd.r(), cmd.g(), cmd.b());

        let position = CoordPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());
        let dimensions = SizePacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());
        let (x, y) = (position.x(), position.y());
        let (width, height) = (dimensions.width(), dimensions.height());
        let rectangle = Rectangle {
            top_left: Vertex {
                color,
                x: i11::new((x & 0x3F0) as i16),
                y: i11::new((y & 0x1FF) as i16),
                u: 0,
                v: 0,
            },
            width: ((width & 0x3FF) + 0xF) & !0xF,
            height: height & 0x1FF,
            transparency: TransparencyMode::Opaque,
            texconfig: None,
        };

        trace!(psx.loggers.gpu, "quick rect fill"; rect = rectangle);
        self.renderer.exec(Command::Draw {
            primitive: Primitive::Rectangle(rectangle),
        });
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

            let mut position =
                VertexPositionPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());

            position.apply_offset(
                psx.gpu.environment.drawing_offset_x,
                psx.gpu.environment.drawing_offset_y,
            );

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

        let tri_1 = [
            vertex_a.to_vertex(),
            vertex_b.to_vertex(),
            vertex_c.to_vertex(),
        ];
        let tri_2 = [
            vertex_b.to_vertex(),
            vertex_c.to_vertex(),
            vertex_d.to_vertex(),
        ];

        let texconfig = cmd.textured().then(|| {
            let clut = vertex_a.uv.clut();
            let texpage = vertex_b.uv.texpage();
            let texconfig = TexConfig {
                clut,
                texpage,
                texwindow: psx.gpu.environment.texwindow,
            };

            let stat = &mut psx.gpu.status;
            stat.set_texpage_x_base(texpage.x_base());
            stat.set_texpage_y_base(texpage.y_base());
            stat.set_blending_mode(texpage.blending_mode());
            stat.set_texpage_depth(texpage.depth());

            if psx.gpu.environment.double_vram {
                stat.set_texture_disable(vertex_b.uv.texture_disable());
            } else {
                stat.set_texture_disable(false);
            }

            texconfig
        });

        let triangle = Triangle {
            vertices: tri_1,
            shading: cmd.shading_mode(),
            transparency: cmd.transparency_mode(),
            texconfig,
        };

        trace!(psx.loggers.gpu, "drawing triangle"; tri = triangle);
        self.renderer.exec(Command::Draw {
            primitive: Primitive::Triangle(triangle),
        });

        if cmd.polygon_mode() == PolygonMode::Rectangle {
            let triangle = Triangle {
                vertices: tri_2,
                shading: cmd.shading_mode(),
                transparency: cmd.transparency_mode(),
                texconfig,
            };

            trace!(psx.loggers.gpu, "drawing triangle"; tri = triangle);
            self.renderer.exec(Command::Draw {
                primitive: Primitive::Triangle(triangle),
            });
        }
    }

    fn renderer_exec_drawing_area(&mut self, psx: &mut PSX) {
        self.renderer.exec(Command::SetDrawingArea(DrawingArea {
            coords: VramCoords {
                x: psx.gpu.environment.drawing_area_top_left_x,
                y: psx.gpu.environment.drawing_area_top_left_y,
            },
            dimensions: VramDimensions {
                width: u11::new(
                    psx.gpu
                        .environment
                        .drawing_area_bottom_right_x
                        .value()
                        .saturating_sub(psx.gpu.environment.drawing_area_top_left_x.value()),
                ),
                height: u10::new(
                    psx.gpu
                        .environment
                        .drawing_area_bottom_right_y
                        .value()
                        .saturating_sub(psx.gpu.environment.drawing_area_top_left_y.value()),
                ),
            },
        }));
    }

    fn renderer_exec_drawing_settings(&mut self, psx: &mut PSX) {
        let stat = &mut psx.gpu.status;
        self.renderer
            .exec(Command::SetDrawingSettings(DrawingSettings {
                blending_mode: stat.blending_mode(),
                write_to_mask: stat.write_to_mask(),
                check_mask: stat.check_mask(),
            }));
    }

    fn exec_drawing_area_top_left(&mut self, psx: &mut PSX, cmd: RenderingCommand) {
        let cmd = cmd.drawing_area_corner_cmd();
        info!(psx.loggers.gpu, "updating drawing area top left"; top_left = cmd.clone());

        psx.gpu.environment.drawing_area_top_left_x = cmd.x();
        psx.gpu.environment.drawing_area_top_left_y = cmd.y();

        self.renderer_exec_drawing_area(psx);
    }

    fn exec_drawing_area_bottom_right(&mut self, psx: &mut PSX, cmd: RenderingCommand) {
        let cmd = cmd.drawing_area_corner_cmd();
        info!(psx.loggers.gpu, "updating drawing area bottom right"; bottom_right = cmd.clone());

        psx.gpu.environment.drawing_area_bottom_right_x = cmd.x();
        psx.gpu.environment.drawing_area_bottom_right_y = cmd.y();

        self.renderer_exec_drawing_area(psx);
    }

    #[expect(clippy::unused_self, reason = "consistency")]
    fn exec_drawing_offset(&mut self, psx: &mut PSX, cmd: RenderingCommand) {
        let cmd = cmd.drawing_offset_cmd();
        info!(psx.loggers.gpu, "updating drawing area offset"; offset = cmd.clone());

        psx.gpu.environment.drawing_offset_x = cmd.x();
        psx.gpu.environment.drawing_offset_y = cmd.y();
    }

    #[expect(clippy::unused_self, reason = "consistency")]
    fn exec_drawing_settings(&mut self, psx: &mut PSX, cmd: RenderingCommand) {
        let settings = cmd.drawing_settings_cmd();
        info!(psx.loggers.gpu, "updating drawing settings"; settings = settings.clone());

        let stat = &mut psx.gpu.status;
        stat.set_texpage_x_base(settings.texpage().x_base());
        stat.set_texpage_y_base(settings.texpage().y_base());
        stat.set_blending_mode(settings.texpage().blending_mode());
        stat.set_texpage_depth(settings.texpage().depth());
        stat.set_compression_mode(settings.compression_mode());
        stat.set_enable_drawing_to_display(settings.enable_drawing_to_display());

        if psx.gpu.environment.double_vram {
            stat.set_texture_disable(settings.texture_disable());
        } else {
            stat.set_texture_disable(false);
        }

        psx.gpu.environment.textured_rect_flip_x = settings.textured_rect_flip_x();
        psx.gpu.environment.textured_rect_flip_y = settings.textured_rect_flip_y();

        self.renderer_exec_drawing_settings(psx);
    }

    #[expect(clippy::unused_self, reason = "consistency")]
    fn exec_texwindow_settings(&mut self, psx: &mut PSX, cmd: RenderingCommand) {
        let settings = cmd.texture_window_settings_cmd();
        info!(psx.loggers.gpu, "updating texwindow settings"; settings = settings.clone());
        psx.gpu.environment.texwindow = settings.texwindow();

        self.renderer
            .exec(Command::SetTexWindow(settings.texwindow()));
    }

    #[expect(clippy::unused_self, reason = "consistency")]
    fn exec_mask_settings(&mut self, psx: &mut PSX, cmd: RenderingCommand) {
        let settings = cmd.mask_settings_cmd();
        info!(psx.loggers.gpu, "updating mask settings"; settings = settings.clone());

        psx.gpu.status.set_write_to_mask(settings.write_to_mask());
        psx.gpu.status.set_check_mask(settings.check_mask());

        self.renderer_exec_drawing_settings(psx);
    }

    fn exec_cpu_to_vram_blit(&mut self, psx: &mut PSX, _: RenderingCommand) {
        let dest = CoordPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());
        let size = SizePacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());

        info!(psx.loggers.gpu, "starting CPU to VRAM blit"; dest = dest.clone(), size = size.clone());
        self.inner = State::CpuToVramBlit { dest, size };

        psx.gpu.status.set_ready_to_send_vram(false);
        psx.scheduler.schedule(Event::DmaUpdate, 0);
    }

    fn exec_vram_to_cpu_blit(&mut self, psx: &mut PSX, _: RenderingCommand) {
        psx.gpu.status.set_ready_to_send_vram(true);

        let src = CoordPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());
        let size = SizePacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());
        info!(psx.loggers.gpu, "starting VRAM to CPU blit"; src = src.clone(), size = size.clone());

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

        let (sender, receiver) = oneshot::channel();
        let copy = CopyFromVram {
            coords: VramCoords {
                x: u10::new(src.x()),
                y: u9::new(src.y()),
            },
            dimensions: VramDimensions {
                width: u11::new(effective_width),
                height: u10::new(effective_height),
            },
            response: sender,
        };
        self.renderer.exec(Command::CopyFromVram(copy));
        let data = receiver.recv().unwrap();

        let packed = data.chunks(4).map(|chunk| {
            let bytes = [
                chunk[0],
                chunk[1],
                chunk.get(2).copied().unwrap_or_default(),
                chunk.get(3).copied().unwrap_or_default(),
            ];

            u32::from_le_bytes(bytes)
        });

        psx.gpu.response_queue.extend(packed);
        psx.scheduler.schedule(Event::DmaUpdate, 0);
    }

    fn exec_rectangle(&mut self, psx: &mut PSX, cmd: RenderingCommand) {
        let cmd = cmd.rectangle_cmd();
        let color = Rgba8::new(cmd.r(), cmd.g(), cmd.b());

        let mut position =
            VertexPositionPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());

        position.apply_offset(
            psx.gpu.environment.drawing_offset_x,
            psx.gpu.environment.drawing_offset_y,
        );

        let (uv, texconfig) = if cmd.textured() {
            let uv = VertexUVPacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());
            let config = TexConfig {
                clut: uv.clut(),
                texpage: psx.gpu.status.texpage(),
                texwindow: psx.gpu.environment.texwindow,
            };

            (uv, Some(config))
        } else {
            (VertexUVPacket::default(), None)
        };

        let (width, height) = match cmd.rectangle_mode() {
            RectangleMode::Variable => {
                let size = SizePacket::from_bits(psx.gpu.render_queue.pop_front().unwrap());
                (size.width(), size.height())
            }
            RectangleMode::SinglePixel => (1, 1),
            RectangleMode::Sprite8 => (8, 8),
            RectangleMode::Sprite16 => (16, 16),
        };

        let rectangle = Rectangle {
            top_left: Vertex {
                color,
                x: position.x(),
                y: position.y(),
                u: uv.u(),
                v: uv.v(),
            },
            width,
            height,
            transparency: cmd.transparency_mode(),
            texconfig,
        };

        trace!(psx.loggers.gpu, "drawing rectangle"; rectangle = rectangle);
        self.renderer.exec(Command::Draw {
            primitive: Primitive::Rectangle(rectangle),
        });
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
                self.inner = State::PolyLine { cmd, received: 0 };
            }
        }
    }

    /// Executes the given rendering command.
    pub fn exec_render(&mut self, psx: &mut PSX, cmd: RenderingCommand) {
        trace!(
            psx.loggers.gpu,
            "executing render cmd: {cmd:?} (0x{:08X})",
            cmd.to_bits()
        );

        match cmd.opcode() {
            RenderingOpcode::Misc => match cmd.misc_opcode().unwrap() {
                MiscOpcode::NOP => trace!(psx.loggers.gpu, "nop"),
                MiscOpcode::ClearCache => trace!(psx.loggers.gpu, "clear cache"),
                MiscOpcode::QuickRectangleFill => self.exec_quick_rect_fill(psx, cmd),
                _ => error!(
                    psx.loggers.gpu,
                    "unimplemented rendering (misc) command: {:?}",
                    cmd.misc_opcode()
                ),
            },
            RenderingOpcode::Environment => match cmd.environment_opcode().unwrap() {
                EnvironmentOpcode::DrawingAreaTopLeft => self.exec_drawing_area_top_left(psx, cmd),
                EnvironmentOpcode::DrawingAreaBottomRight => {
                    self.exec_drawing_area_bottom_right(psx, cmd)
                }
                EnvironmentOpcode::DrawingOffset => self.exec_drawing_offset(psx, cmd),
                EnvironmentOpcode::DrawingSettings => self.exec_drawing_settings(psx, cmd),
                EnvironmentOpcode::TexWindowSettings => self.exec_texwindow_settings(psx, cmd),
                EnvironmentOpcode::MaskSettings => self.exec_mask_settings(psx, cmd),
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
