use crate::tab::{Context, Tab};
use eframe::egui::{self, Ui};

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

    fn ui(&mut self, ui: &mut Ui, _ctx: Context) {
        ui.label("hello");
    }

    fn closable(&mut self) -> bool {
        false
    }
}
