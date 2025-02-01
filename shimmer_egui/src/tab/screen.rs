use crate::tab::{Context, Tab};
use eframe::{
    egui::{self, Ui, Vec2},
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
        // let aspect_ratio = 1.333; // 4:3
        let aspect_ratio = 2.0; // 2:1

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
                renderer: ctx.exclusive.renderer.clone(),
            },
        ));
    }

    fn closable(&mut self) -> bool {
        false
    }
}
