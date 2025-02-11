use super::WindowUi;
use crate::ExclusiveState;
use eframe::egui::{self, Align, Id, RichText, Ui, Vec2, Window};
use shimmer::core::cpu::FREQUENCY;

pub struct Control {
    _id: Id,
}

impl Control {
    pub fn new(id: Id) -> Self
    where
        Self: Sized,
    {
        Self { _id: id }
    }
}

impl WindowUi for Control {
    fn build<'open>(&mut self, open: &'open mut bool) -> Window<'open> {
        Window::new("Control")
            .open(open)
            .resizable(false)
            .default_size(Vec2::new(0.0, 0.0))
    }

    fn show(&mut self, state: &mut ExclusiveState, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut state.controls.running, "Run");
            if ui
                .add_enabled(!state.controls.running, egui::Button::new("Cycle"))
                .clicked()
            {
                state.emulator.cycle_for(1);
            }
        });

        ui.horizontal(|ui| {
            ui.label("Emulated:");
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                ui.label(format!("{:.3?}", state.timing.emulated_time));
            });
        });

        ui.horizontal(|ui| {
            ui.label("Elapsed:");
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                ui.label(format!("{:.3?}", state.timing.running_timer.elapsed()));
            });
        });

        ui.separator();

        let current = state.timing.running_timer.scale();
        ui.horizontal(|ui| {
            let mut scale = state.timing.running_timer.scale();
            ui.label("Scale:");
            if ui
                .add(
                    egui::DragValue::new(&mut scale)
                        .speed(0.001)
                        .range(0.0..=10.0)
                        .min_decimals(9)
                        .max_decimals(9),
                )
                .changed()
            {
                state.timing.running_timer.set_scale(scale);
            }
        });

        ui.horizontal(|ui| {
            if ui.button("x0.1").clicked() {
                state.timing.running_timer.set_scale(current * 0.1);
            }

            if ui.button("x0.5").clicked() {
                state.timing.running_timer.set_scale(current * 0.5);
            }

            if ui.button("x2").clicked() {
                state.timing.running_timer.set_scale(current * 2.0);
            }

            if ui.button("x10").clicked() {
                state.timing.running_timer.set_scale(current * 10.0);
            }
        });

        let cpu_freq: si_scale::value::Value = (FREQUENCY as f64 * current).into();
        ui.label(
            RichText::new(format!(
                "Equivalent of running at ~ {}Hz",
                si_scale::format_value!(cpu_freq, "{:.3}")
            ))
            .small(),
        );
    }
}
