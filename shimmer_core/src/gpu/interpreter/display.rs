use crate::{
    PSX,
    gpu::{
        Interpreter, Status,
        cmd::{DisplayCommand, DisplayOpcode},
        renderer::{Command, DisplayResolution, DisplayTopLeft},
    },
    scheduler::Event,
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

                self.renderer
                    .exec(Command::SetDisplayResolution(DisplayResolution {
                        horizontal: settings.horizontal_resolution(),
                        vertical: settings.vertical_resolution(),
                    }));
            }
            DisplayOpcode::DmaDirection => {
                let cmd = cmd.dma_direction_cmd();
                psx.gpu.status.set_dma_direction(cmd.direction());
                psx.scheduler.schedule(Event::DmaUpdate, 0);
            }
            DisplayOpcode::DisplayArea => {
                let settings = cmd.display_area_cmd();
                psx.gpu.display.top_left_x = settings.x();
                psx.gpu.display.top_left_y = settings.y();

                self.renderer
                    .exec(Command::SetDisplayTopLeft(DisplayTopLeft {
                        x: settings.x(),
                        y: settings.y(),
                    }));
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
            DisplayOpcode::AcknowledgeGpuInterrupt => {
                psx.gpu.status.set_interrupt_request(false);
            }
            DisplayOpcode::ResetCommandBuffer => {
                warn!(psx.loggers.gpu, "reset command buffer stub");
            }
            _ => error!(psx.loggers.gpu, "unimplemented display command: {cmd:?}"),
        }
    }
}
