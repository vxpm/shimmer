use super::WindowUi;
use crate::ExclusiveState;
use eframe::egui::{self, Id, Ui, Vec2, Window};
use egui_plot::{Legend, Line, Plot, PlotPoints};
use shimmer_core::{cpu, sio0::Snapshot};

pub struct Serial {
    offset: f64,
    _id: Id,
}

impl Serial {
    pub fn new(id: Id) -> Self
    where
        Self: Sized,
    {
        Self {
            // offset: 300.9f64,
            offset: 167.1,
            _id: id,
        }
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
        ui.add(egui::Slider::new(&mut self.offset, 0.0..=50000.0));

        let snaps = &state.emulator.psx().sio0.snaps;
        let mut index = 0usize;

        let mut points = |f: fn(&Snapshot) -> bool| -> PlotPoints {
            let result = snaps
                .iter()
                .filter(|s| s.cycle as f64 / (cpu::CYCLES_1_MS as f64) >= self.offset)
                .map(|s| {
                    [
                        s.cycle as f64 / (cpu::CYCLES_1_MS as f64),
                        !f(s) as u64 as f64 / 2.0 + index as f64,
                    ]
                })
                .collect();

            index += 1;
            result
        };

        let cs = points(|s| s.control.selected());
        let tx = points(|s| s.tx.is_some());
        let rx = points(|s| s.rx.is_some());
        let ack = points(|s| s.status.device_ready_to_receive());
        let irq = points(|s| s.status.interrupt_request());

        let cs = Line::new(cs).name("cs");
        let tx = Line::new(tx).name("tx");
        let rx = Line::new(rx).name("rx");
        let ack = Line::new(ack).name("ack");
        let irq = Line::new(irq).name("irq");

        Plot::new("my_plot")
            .legend(Legend::default())
            .x_axis_label("ms")
            .y_axis_label("logic level")
            .view_aspect(2.0)
            .show(ui, |plot_ui| {
                plot_ui.line(irq);
                plot_ui.line(ack);
                plot_ui.line(rx);
                plot_ui.line(tx);
                plot_ui.line(cs);
            });
    }
}
