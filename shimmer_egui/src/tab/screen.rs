use crate::tab::{Context, Tab};
use eframe::{
    egui::{self, Ui, Vec2},
    egui_wgpu::{self, CallbackTrait},
};
use shimmer_wgpu_compute::WgpuRenderer;

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

pub struct Screen {}

impl Tab for Screen {
    fn new(_: u64) -> Self
    where
        Self: Sized,
    {
        Self {}
    }

    fn title(&mut self) -> egui::WidgetText {
        "Video Output".into()
    }

    fn ui(&mut self, ui: &mut Ui, ctx: Context) {
        let rect = if ui.available_width() < ui.available_height() {
            ui.allocate_exact_size(
                Vec2::new(ui.available_width(), ui.available_width() / 1.333),
                egui::Sense::click(),
            )
            .0
        } else {
            ui.allocate_exact_size(
                Vec2::new(ui.available_height() * 1.333, ui.available_height()),
                egui::Sense::click(),
            )
            .0
        };

        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
            rect,
            RendererCallback {
                renderer: ctx.exclusive.renderer.clone(),
            },
        ));
    }

    fn closable(&mut self) -> bool {
        false
    }
}
