use super::WindowUi;
use crate::ExclusiveState;
use eframe::egui::{self, Id, Ui, Vec2, Window};
use egui_plot::{Legend, Line, Plot, PlotPoints};
use shimmer_core::{cpu, sio0::Snapshot};

pub struct Serial {
    offset: f64,
    id: Id,
}

impl Serial {
    pub fn new(id: Id) -> Self
    where
        Self: Sized,
    {
        Self { offset: 0.0f64, id }
    }
}

impl WindowUi for Serial {
    fn build<'open>(&mut self, open: &'open mut bool) -> Window<'open> {
        Window::new("SIO0 Debug")
            .open(open)
            .min_width(200.0)
            .max_height(0.0)
            .default_size(Vec2::new(0.0, 0.0))
    }

    fn show(&mut self, state: &mut ExclusiveState, ui: &mut Ui) {
        ui.add(egui::Slider::new(&mut self.offset, 0.0..=500.0));

        let snaps = &state.emulator.psx().sio0.snaps;
        let points = |f: fn(&Snapshot) -> bool| -> PlotPoints {
            snaps
                .iter()
                .filter(|s| s.cycle as f64 / (cpu::CYCLES_1_MS as f64) >= self.offset)
                .map(|s| {
                    [
                        s.cycle as f64 / (cpu::CYCLES_1_MS as f64),
                        f(s) as u64 as f64,
                    ]
                })
                .collect()
        };

        let ack = points(|s| s.status.device_ready_to_receive());
        let rx = points(|s| s.rx.is_some());
        let tx = points(|s| s.tx.is_some());

        let ack = Line::new(ack).name("ack");
        let rx = Line::new(rx).name("rx");
        let tx = Line::new(tx).name("tx");

        Plot::new("my_plot")
            .legend(Legend::default())
            .view_aspect(2.0)
            .show(ui, |plot_ui| {
                plot_ui.line(ack);
                plot_ui.line(rx);
                plot_ui.line(tx);
            });
    }
}
