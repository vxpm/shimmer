use crate::{
    PSX,
    gpu::{
        Interpreter,
        interface::{Command, DisplayResolution, VramCoords},
    },
    scheduler::Event,
};
use shimmer_core::gpu::{
    Status,
    cmd::{
        DisplayCommand, DisplayOpcode,
        environment::{DrawingAreaCornerCmd, DrawingOffsetCmd},
    },
};
use tinylog::{error, trace, warn};

impl Interpreter {
    /// Executes the given display command.
    pub fn exec_display(&mut self, psx: &mut PSX, cmd: DisplayCommand) {
        trace!(psx.loggers.gpu, "received display cmd: {cmd:?}");

        match cmd.opcode().unwrap() {
            DisplayOpcode::ResetGpu => {
                // TODO: reset internal registers
                psx.gpu.status = Status::default();
                psx.gpu.render_queue.clear();
            }
            DisplayOpcode::DisplayMode => {
                let cmd = cmd.display_mode_cmd();
                let stat = &mut psx.gpu.status;

                stat.set_horizontal_resolution(cmd.horizontal_resolution());
                stat.set_vertical_resolution(cmd.vertical_resolution());
                stat.set_video_mode(cmd.video_mode());
                stat.set_display_depth(cmd.display_depth());
                stat.set_vertical_interlace(cmd.vertical_interlace());
                stat.set_force_horizontal_368(cmd.force_horizontal_368());
                stat.set_flip_screen_x(cmd.flip_screen_x());

                self.renderer
                    .exec(Command::SetDisplayResolution(DisplayResolution {
                        horizontal: cmd.horizontal_resolution(),
                        vertical: cmd.vertical_resolution(),
                    }));
            }
            DisplayOpcode::DmaDirection => {
                let cmd = cmd.dma_direction_cmd();
                psx.gpu.status.set_dma_direction(cmd.direction());
                psx.scheduler.schedule(Event::DmaUpdate, 0);
            }
            DisplayOpcode::DisplayArea => {
                let cmd = cmd.display_area_cmd();
                psx.gpu.display.top_left_x = cmd.x();
                psx.gpu.display.top_left_y = cmd.y();

                self.renderer.exec(Command::SetDisplayTopLeft(VramCoords {
                    x: cmd.x(),
                    y: cmd.y(),
                }));
            }
            DisplayOpcode::HorizontalDisplayRange => {
                let cmd = cmd.horizontal_display_range_cmd();
                psx.gpu.display.horizontal_range = cmd.x1()..cmd.x2();
            }
            DisplayOpcode::VerticalDisplayRange => {
                let cmd = cmd.vertical_display_range_cmd();
                psx.gpu.display.vertical_range = cmd.y1()..cmd.y2();
            }
            DisplayOpcode::DisplayEnabled => {
                let cmd = cmd.display_enable_cmd();
                psx.gpu.status.set_disable_display(cmd.disabled());
            }
            DisplayOpcode::VramSizeV2 => {
                let cmd = cmd.vram_size_cmd();
                psx.gpu.environment.double_vram = cmd.double();
            }
            DisplayOpcode::AcknowledgeGpuInterrupt => {
                psx.gpu.status.set_interrupt_request(false);
            }
            DisplayOpcode::ResetCommandBuffer => {
                warn!(psx.loggers.gpu, "reset command buffer");
                psx.gpu.render_queue.clear();
            }
            DisplayOpcode::ReadGpuRegister => {
                let index = cmd.to_bits() & 0b111;
                match index {
                    0 | 1 | 6 | 7 => (),
                    2 => todo!(),
                    3 => {
                        psx.gpu.response_queue.push_front(
                            DrawingAreaCornerCmd::from_bits(0)
                                .with_x(psx.gpu.environment.drawing_area_top_left_x)
                                .with_y(psx.gpu.environment.drawing_area_top_left_y)
                                .to_bits(),
                        );
                    }
                    4 => {
                        psx.gpu.response_queue.push_front(
                            DrawingAreaCornerCmd::from_bits(0)
                                .with_x(psx.gpu.environment.drawing_area_bottom_right_x)
                                .with_y(psx.gpu.environment.drawing_area_bottom_right_y)
                                .to_bits(),
                        );
                    }
                    5 => {
                        psx.gpu.response_queue.push_front(
                            DrawingOffsetCmd::from_bits(0)
                                .with_x(psx.gpu.environment.drawing_offset_x)
                                .with_y(psx.gpu.environment.drawing_offset_y)
                                .to_bits(),
                        );
                    }
                    _ => unreachable!(),
                }
            }
            _ => error!(psx.loggers.gpu, "unimplemented display command: {cmd:?}"),
        }
    }
}
