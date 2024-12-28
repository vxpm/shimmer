use crate::{TabContext, VioletTab};
use eframe::egui::{self, Ui};

pub struct Screen {}

impl VioletTab for Screen {
    fn new(_: u64) -> Self
    where
        Self: Sized,
    {
        Self {}
    }

    fn title(&mut self) -> egui::WidgetText {
        "Video Output".into()
    }

    fn ui(&mut self, ui: &mut Ui, _ctx: TabContext) {
        ui.label("hello");
    }

    fn closable(&mut self) -> bool {
        false
    }
}
