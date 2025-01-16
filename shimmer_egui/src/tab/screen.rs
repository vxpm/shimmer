use std::sync::Arc;

use crate::tab::{Context, Tab};
use eframe::{
    egui::{self, Ui},
    egui_wgpu::{self, CallbackTrait},
};
use parking_lot::Mutex;
use shimmer_wgpu::Renderer;

pub struct RendererCallback {
    renderer: Arc<Mutex<Renderer>>,
}

impl CallbackTrait for RendererCallback {
    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut eframe::wgpu::RenderPass<'static>,
        _callback_resources: &egui_wgpu::CallbackResources,
    ) {
        self.renderer.lock().render(render_pass);
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
        let (rect, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::click());
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
