use super::WindowUi;
use crate::State;
use eframe::{
    egui::{self, Id, Ui, Vec2, Window},
    egui_wgpu::{self, CallbackTrait},
};
use shimmer_wgpu::WgpuRenderer;

pub struct RendererCallback {
    renderer: WgpuRenderer,
    vram: bool,
}

impl CallbackTrait for RendererCallback {
    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut eframe::wgpu::RenderPass<'static>,
        _callback_resources: &egui_wgpu::CallbackResources,
    ) {
        if self.vram {
            self.renderer.render_vram(render_pass);
        } else {
            self.renderer.render_display(render_pass);
        }
    }
}

pub struct Display {
    _id: Id,
    vram: bool,
}

impl Display {
    pub fn new(id: Id, vram: bool) -> Self
    where
        Self: Sized,
    {
        Self { _id: id, vram }
    }
}

impl WindowUi for Display {
    fn build<'open>(&mut self, open: &'open mut bool) -> Window<'open> {
        let title = if self.vram { "VRAM" } else { "Display" };
        let min_size = if self.vram {
            Vec2::new(200.0, 100.0)
        } else {
            Vec2::new(200.0, 150.0)
        };

        Window::new(title)
            .open(open)
            .fade_in(false)
            .fade_out(false)
            .min_size(min_size)
    }

    fn show(&mut self, state: &mut State, ui: &mut Ui) {
        let frame_response = egui::Frame::canvas(ui.style()).show(ui, |ui| {
            let aspect_ratio = if self.vram { 2.0 } else { 4.0 / 3.0 };
            let available_height = ui.available_height() - 20.0;

            let rect = if ui.available_width() < available_height {
                ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), ui.available_width() / aspect_ratio),
                    egui::Sense::click(),
                )
                .0
            } else {
                ui.allocate_exact_size(
                    Vec2::new(available_height * aspect_ratio, available_height),
                    egui::Sense::click(),
                )
                .0
            };

            let position = ui
                .input(|i| i.pointer.latest_pos())
                .map(|pos| (pos - rect.min) / rect.size());

            ui.painter().add(egui_wgpu::Callback::new_paint_callback(
                rect,
                RendererCallback {
                    renderer: state.renderer.clone(),
                    vram: self.vram,
                },
            ));

            position
        });

        state.input.update(ui.ctx(), state.emulator.joypad_mut());

        if self.vram
            && frame_response.response.hovered()
            && let Some(pos) = frame_response.inner
            && pos.x > 0.0
            && pos.y > 0.0
        {
            let vram_pos = (pos * Vec2::new(1024.0, 512.0)).round();
            ui.label(format!("mouse at: {:?}", vram_pos));
        }
    }
}
