use super::WindowUi;
use crate::ExclusiveState;
use eframe::egui::{self, Color32, Id, RichText, Ui, Window};
use egui_extras::{Column, TableBuilder, TableRow};
use tinylog::{Level, logger::Context as LoggerContext};

const MAX_RECORDS_SHOWN: usize = 50_000;

pub struct LogViewer {
    _id: Id,
    row_heights: Vec<f32>,

    // user settings
    logger_ctx: LoggerContext,
}

impl LogViewer {
    fn draw_header(&mut self, state: &mut ExclusiveState, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(format!(
                "Records: {} (max shown: {})",
                state.log_records.len(self.logger_ctx.clone()),
                MAX_RECORDS_SHOWN
            ));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let current_level = state.log_family.level_of(&self.logger_ctx).unwrap();
                let mut current_level_index = current_level as usize;
                egui::ComboBox::from_label("Level:")
                    .selected_text(current_level.to_string())
                    .show_index(
                        ui,
                        &mut current_level_index,
                        Level::Error as usize + 1,
                        |i| unsafe { std::mem::transmute::<u8, Level>(i as u8) }.to_string(),
                    );

                let new_level =
                    unsafe { std::mem::transmute::<u8, Level>(current_level_index as u8) };
                if new_level != current_level {
                    state
                        .log_family
                        .set_level_of(&self.logger_ctx, new_level)
                        .unwrap();
                }

                let response = egui::ComboBox::from_label("Context:")
                    .selected_text(self.logger_ctx.to_string())
                    .show_ui(ui, |ui| {
                        let mut changed = false;
                        for context in state.log_family.contexts() {
                            let context_str = context.to_string();
                            if ui
                                .selectable_value(&mut self.logger_ctx, context, context_str)
                                .clicked()
                            {
                                changed = true;
                            }
                        }

                        changed
                    });

                if response.inner.unwrap_or_default() {
                    self.row_heights.clear();
                }
            });
        });
    }

    fn draw_logs(&mut self, state: &mut ExclusiveState, ui: &mut Ui) {
        ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();

        self.row_heights
            .resize(state.log_records.len(self.logger_ctx.clone()), 10.0);

        let heights =
            self.row_heights[self.row_heights.len().saturating_sub(MAX_RECORDS_SHOWN)..].to_owned();
        TableBuilder::new(ui)
            .auto_shrink([false; 2])
            .stick_to_bottom(true)
            .striped(true)
            .column(Column::auto().at_least(110.0))
            .column(Column::auto().at_least(50.0))
            .column(Column::auto().at_least(90.0))
            .column(Column::remainder())
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label("Time");
                });

                header.col(|ui| {
                    ui.label("Level");
                });

                header.col(|ui| {
                    ui.label("Context");
                });

                header.col(|ui| {
                    ui.label("Record");
                });
            })
            .body(|body| {
                body.heterogeneous_rows(heights.into_iter(), |mut row| {
                    self.draw_row(state, &mut row);
                });
            });
    }

    fn draw_row(&mut self, state: &mut ExclusiveState, row: &mut TableRow) {
        let index = row.index();
        let offset = self.row_heights.len().saturating_sub(MAX_RECORDS_SHOWN);

        let log = state
            .log_records
            .get(&self.logger_ctx, offset + index)
            .unwrap();

        row.col(|ui| {
            ui.label(
                RichText::new(log.value.time().format("%H:%M:%S%.6f").to_string())
                    .monospace()
                    .weak(),
            );
        });

        row.col(|ui| {
            let color = match log.value.static_data.level {
                tinylog::Level::Trace => Color32::LIGHT_GRAY,
                tinylog::Level::Debug => Color32::LIGHT_BLUE,
                tinylog::Level::Info => Color32::LIGHT_GREEN,
                tinylog::Level::Warn => Color32::LIGHT_YELLOW,
                tinylog::Level::Error => Color32::LIGHT_RED,
            };

            ui.label(
                RichText::new(log.value.static_data.level.to_string())
                    .monospace()
                    .color(color)
                    .strong(),
            );
        });

        row.col(|ui| {
            ui.label(RichText::new(log.ctx.to_string()).monospace());
        });

        row.col(|ui| {
            ui.set_max_width(ui.available_width());
            ui.set_min_width(0.0);

            let height = ui
                .vertical(|ui| {
                    ui.label(format!("{}", log.value.message));
                    for attachment in &log.value.attachments {
                        let key = &attachment.key;
                        let value = if let Some(display) = attachment.value.as_display() {
                            display.to_string()
                        } else if let Some(debug) = attachment.value.as_debug() {
                            format!("{debug:?}")
                        } else {
                            "(opaque)".to_string()
                        };

                        ui.label(RichText::new(format!("{key:?}: {}", value)).small().weak());
                    }
                })
                .response
                .rect
                .height();

            self.row_heights[offset + index] = height;
        });
    }
}

impl LogViewer {
    pub fn new(id: Id) -> Self
    where
        Self: Sized,
    {
        Self {
            _id: id,
            row_heights: Vec::new(),

            logger_ctx: LoggerContext::new("psx"),
        }
    }
}

impl WindowUi for LogViewer {
    fn build<'open>(&mut self, open: &'open mut bool) -> egui::Window<'open> {
        Window::new("Logs").open(open)
    }

    fn show(&mut self, state: &mut ExclusiveState, ui: &mut Ui) {
        ui.vertical(|ui| {
            self.draw_header(state, ui);
            ui.separator();
            self.draw_logs(state, ui);
        });
    }
}
