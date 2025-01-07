use crate::{
    PSX,
    gpu::{
        DmaDirection, GpuStatus,
        instr::{DisplayOpcode, EnvironmentOpcode, Instruction, MiscOpcode, RenderingOpcode},
    },
};
use tinylog::debug;

pub struct Interpreter<'psx> {
    pub psx: &'psx mut PSX,
}

impl<'psx> Interpreter<'psx> {
    pub fn new(psx: &'psx mut PSX) -> Self {
        Self { psx }
    }

    /// Executes the given GPU instruction.
    pub fn exec(&mut self, instr: Instruction) {
        debug!(self.psx.loggers.gpu, "received instr: {instr:?}");

        match instr {
            Instruction::Rendering(instr) => match instr.opcode() {
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
                            stat.set_enable_drawing_to_display(
                                settings.enable_drawing_to_display(),
                            );
                            stat.set_texpage_y_base_2(settings.texpage_y_base_2());

                            self.psx.gpu.textured_rect_flip_x = settings.textured_rect_flip_x();
                            self.psx.gpu.textured_rect_flip_y = settings.textured_rect_flip_y();
                        }
                        _ => debug!(self.psx.loggers.gpu, "unimplemented"),
                    }
                }
                _ => debug!(self.psx.loggers.gpu, "unimplemented"),
            },
            Instruction::Display(instr) => match instr.opcode().unwrap() {
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
                _ => debug!(self.psx.loggers.gpu, "unimplemented"),
            },
        }
    }

    /// Executes all queued GPU instructions.
    pub fn exec_queued(&mut self) {
        while !self.psx.gpu.queue.is_empty() {
            let instr = self.psx.gpu.queue.pop_front().unwrap();
            self.exec(instr);
        }
    }
}
