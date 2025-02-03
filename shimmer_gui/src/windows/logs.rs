use super::WindowUi;
use crate::ExclusiveState;
use eframe::egui::{
    self, Color32, Id, RichText, Ui, UiBuilder, Vec2, Window, style::ScrollAnimation,
};
use egui_extras::{Column, TableBuilder, TableRow};
use std::{cell::RefCell, collections::BTreeMap};
use tinylog::{Level, logger::Context as LoggerContext, record::RecordWithCtx};

// fn header_cell_ui(&mut self, ui: &mut Ui, cell: &egui_table::HeaderCellInfo) {
//     let egui_table::HeaderCellInfo { col_range, .. } = cell;
//
//     egui::Frame::none()
//         .inner_margin(egui::Margin::symmetric(4.0, 0.0))
//         .show(ui, |ui| match col_range.start {
//             0 => {
//                 ui.label("TIME");
//             }
//             1 => {
//                 ui.label("LEVEL");
//             }
//             2 => {
//                 ui.label("CONTEXT");
//             }
//             3 => {
//                 ui.label("MESSAGE");
//             }
//             _ => unreachable!(),
//         });
// }
//
// fn cell_ui(&mut self, ui: &mut Ui, cell: &egui_table::CellInfo) {
//     let egui_table::CellInfo { row_nr, col_nr, .. } = *cell;
//
//     if row_nr % 2 == 0 {
//         ui.painter()
//             .rect_filled(ui.max_rect(), 0.0, ui.visuals().faint_bg_color);
//     }
//
//     let Some(record) = &self.prefetched.get(&row_nr) else {
//         return;
//     };
//
//     egui::Frame::none()
//         .inner_margin(egui::Margin::symmetric(4.0, 4.0))
//         .show(ui, |ui| match col_nr {
//             0 => {
//                 ui.vertical(|ui| {
//                     ui.label(
//                         RichText::new(
//                             record.value.time().format("%F %H:%M:%S%.3f").to_string(),
//                         )
//                         .monospace()
//                         .weak(),
//                     );
//                 });
//             }
//             1 => {
//                 ui.vertical(|ui| {
//                     let color = match record.value.static_data.level {
//                         tinylog::Level::Trace => Color32::LIGHT_GRAY,
//                         tinylog::Level::Debug => Color32::LIGHT_BLUE,
//                         tinylog::Level::Info => Color32::LIGHT_GREEN,
//                         tinylog::Level::Warn => Color32::LIGHT_YELLOW,
//                         tinylog::Level::Error => Color32::LIGHT_RED,
//                     };
//
//                     ui.label(
//                         RichText::new(record.value.static_data.level.to_string())
//                             .monospace()
//                             .color(color)
//                             .strong(),
//                     );
//                 });
//             }
//             2 => {
//                 ui.vertical(|ui| {
//                     ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
//                     ui.label(RichText::new(record.ctx.to_string()).monospace());
//                 });
//             }
//             3 => {
//                 ui.vertical(|ui| {
//                     let (_, space) = ui.allocate_space(Vec2::new(self.message_width, ROW_SIZE));
//                     let response = ui.scope_builder(UiBuilder::new().max_rect(space), |ui| {
//                         ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
//                         ui.label(record.value.message.to_string());
//
//                         if !record.value.attachments.is_empty() {
//                             for attachment in &record.value.attachments {
//                                 let key = &attachment.key;
//                                 let value = if let Some(display) = attachment.value.as_display()
//                                 {
//                                     display.to_string()
//                                 } else if let Some(debug) = attachment.value.as_debug() {
//                                     format!("{debug:?}")
//                                 } else {
//                                     "(opaque)".to_string()
//                                 };
//
//                                 ui.label(
//                                     RichText::new(format!("{key:?}: {}", value)).small().weak(),
//                                 );
//                             }
//                         }
//                     });
//
//                     let size = 8.0 + response.response.rect.size().y;
//                     let old = self.row_heights.insert(cell.row_nr, size);
//
//                     if old != Some(size) {
//                         let mut row_top_offsets = self.row_top_offsets.borrow_mut();
//                         row_top_offsets[cell.row_nr as usize..]
//                             .iter_mut()
//                             .for_each(|o| *o = None);
//                     }
//                 });
//             }
//             _ => unreachable!(),
//         });
// }

pub struct LogViewer {
    id: Id,
    row_heights: Vec<f32>,

    // user settings
    logger_ctx: LoggerContext,
}

impl LogViewer {
    fn draw_header(&mut self, state: &mut ExclusiveState, ui: &mut Ui) {
        ui.horizontal(|ui| {
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

        let heights = self.row_heights.clone();
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
        let log = state.log_records.get(&self.logger_ctx, index).unwrap();

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

            self.row_heights[index] = height;
        });
    }
}

impl LogViewer {
    pub fn new(id: Id) -> Self
    where
        Self: Sized,
    {
        Self {
            id,
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
