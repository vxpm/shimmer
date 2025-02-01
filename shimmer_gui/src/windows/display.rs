use super::WindowUi;
use crate::ExclusiveState;
use eframe::{
    egui::{self, Id, Ui, Vec2, Window},
    egui_wgpu::{self, CallbackTrait},
};
use shimmer_wgpu::WgpuRenderer;

pub struct RendererCallback {
    renderer: WgpuRenderer,
}

impl CallbackTrait for RendererCallback {
    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut eframe::wgpu::RenderPass<'static>,
        _callback_resources: &egui_wgpu::CallbackResources,
    ) {
        self.renderer.render(render_pass);
    }
}

pub struct Display {
    _id: Id,
}

impl WindowUi for Display {
    fn new(id: Id) -> Self
    where
        Self: Sized,
    {
        Self { _id: id }
    }

    fn build<'open>(&mut self, open: &'open mut bool) -> Window<'open> {
        Window::new("Display")
            .open(open)
            .fade_in(false)
            .fade_out(false)
            .min_size(Vec2::new(200.0, 150.0))
    }

    fn show(&mut self, state: &mut ExclusiveState, ui: &mut Ui) {
        let aspect_ratio = 1.333; // 4:3
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
            },
        ));
    }
}
