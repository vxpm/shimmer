use crate::tab::{Context, Tab};
use eframe::{
    egui::{self, RichText, Ui, UiBuilder},
    emath::Vec2b,
    epaint::{Color32, Vec2},
};

pub struct Terminal {}

impl Tab for Terminal {
    fn new(_: u64) -> Self
    where
        Self: Sized,
    {
        Self {}
    }

    fn title(&mut self) -> egui::WidgetText {
        "Terminal".into()
    }

    fn ui(&mut self, ui: &mut Ui, ctx: Context) {
        let available_size = ui.available_size();
        let (rect, _) =
            ui.allocate_exact_size(available_size, egui::Sense::focusable_noninteractive());

        ui.painter().rect(
            rect,
            egui::Rounding::default().at_least(4.0),
            ui.style().visuals.extreme_bg_color,
            egui::Stroke::new(0.25, Color32::LIGHT_GRAY),
        );

        let mut ui = ui.new_child(UiBuilder::new().max_rect(rect.shrink2(Vec2::splat(5.0))));
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .auto_shrink(Vec2b::new(false, false))
            .show(&mut ui, |ui| {
                ui.label(
                    RichText::new(&ctx.exclusive.psx.memory.kernel_stdout)
                        .monospace()
                        .color(Color32::LIGHT_GRAY),
                );
            });
    }
}
