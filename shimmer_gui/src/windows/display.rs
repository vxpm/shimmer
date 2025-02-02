use super::WindowUi;
use crate::ExclusiveState;
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

    fn show(&mut self, state: &mut ExclusiveState, ui: &mut Ui) {
        let aspect_ratio = if self.vram { 2.0 } else { 4.0 / 3.0 };
        let rect = if ui.available_width() < ui.available_height() {
            ui.allocate_exact_size(
                Vec2::new(ui.available_width(), ui.available_width() / aspect_ratio),
                egui::Sense::click(),
            )
            .0
        } else {
            ui.allocate_exact_size(
                Vec2::new(ui.available_height() * aspect_ratio, ui.available_height()),
                egui::Sense::click(),
            )
            .0
        };

        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
            rect,
            RendererCallback {
                renderer: state.renderer.clone(),
                vram: self.vram,
            },
        ));
    }
}
