use crate::{
    colors::LIGHT_PURPLE,
    tab::{Context, Tab},
    util::character_dimensions,
};
use eframe::{
    egui::{self, style::ScrollStyle, RichText, Ui},
    epaint::Color32,
};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use shimmer_core::cpu::Reg;
use strum::VariantArray;

pub struct SystemControl {
    commonmark_cache: CommonMarkCache,
}

impl Tab for SystemControl {
    fn new(_: u64) -> Self
    where
        Self: Sized,
    {
        Self {
            commonmark_cache: CommonMarkCache::default(),
        }
    }

    fn title(&mut self) -> egui::WidgetText {
        "System Control".into()
    }

    fn ui(&mut self, ui: &mut Ui, ctx: Context) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut ctx.exclusive.controls.running, "Run");
            ui.label(format!(
                "{:.3?}/{:.3?}",
                ctx.exclusive.timing.emulated_time,
                ctx.exclusive.timing.running_timer.elapsed()
            ));
        });

        ui.horizontal(|ui| {
            let mut scale = ctx.exclusive.timing.running_timer.scale();
            ui.label("Scale:");
            if ui
                .add(
                    egui::DragValue::new(&mut scale)
                        .speed(0.01)
                        .range(0.0..=5.0),
                )
                .changed()
            {
                ctx.exclusive.timing.running_timer.set_scale(scale);
            }

            if ui.button("0.5x").clicked() {
                ctx.exclusive.timing.running_timer.set_scale(0.5);
            }

            if ui.button("1x").clicked() {
                ctx.exclusive.timing.running_timer.set_scale(1.0);
            }

            if ui.button("2x").clicked() {
                ctx.exclusive.timing.running_timer.set_scale(2.0);
            }
        });

        ui.horizontal(|ui| {
            if ui
                .add_enabled(!ctx.exclusive.controls.running, egui::Button::new("Step"))
                .clicked()
            {
                ctx.exclusive.psx.cycle();
            }

            ui.checkbox(
                &mut ctx.exclusive.controls.alternative_names,
                "Alternative Register Names",
            );
        });

        let (_, monospace_height) = character_dimensions(ui, egui::TextStyle::Monospace, 'A');
        ui.style_mut().spacing.scroll = ScrollStyle::thin();
        egui::ScrollArea::vertical()
            .enable_scrolling(ctx.is_focused)
            .show_rows(
                ui,
                monospace_height,
                Reg::VARIANTS.len() / 2,
                |ui, show_range| {
                    egui::Grid::new("registers").show(ui, |ui| {
                        for chunk in
                            Reg::VARIANTS[show_range.start * 2..show_range.end * 2].chunks(2)
                        {
                            for reg in chunk {
                                let value = ctx.exclusive.psx.cpu.regs().read(*reg);
                                let name = if ctx.exclusive.controls.alternative_names {
                                    RichText::new(reg.alt_name())
                                } else {
                                    RichText::new(format!("{:?}", reg))
                                };
                                let description = reg.description();

                                let response =
                                    ui.label(name.monospace().color(Color32::LIGHT_BLUE));
                                response.on_hover_ui(|ui| {
                                    CommonMarkViewer::new().show(
                                        ui,
                                        &mut self.commonmark_cache,
                                        description,
                                    );
                                });

                                ui.label(
                                    RichText::new(format!("{:08X}", value))
                                        .monospace()
                                        .color(LIGHT_PURPLE),
                                );
                            }
                            ui.end_row();
                        }
                    });
                },
            );
    }
}
