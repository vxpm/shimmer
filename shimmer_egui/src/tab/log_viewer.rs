use crate::tab::{Context, Tab};
use eframe::egui::{self, style::ScrollAnimation, Color32, RichText, Ui};
use egui_table::TableDelegate;
use std::collections::BTreeMap;
use tinylog::{logger::Context as LoggerContext, record::RecordWithCtx};

const ROW_SIZE: f32 = 25.0;

struct LogTableDelegate<'a> {
    ctx: Context<'a>,
    logger_ctx: &'a LoggerContext,
    extra_row_heights: &'a mut BTreeMap<u64, f32>,
    prefetched: &'a mut Vec<RecordWithCtx>,
    prefetched_offset: usize,
}

impl TableDelegate for LogTableDelegate<'_> {
    fn prepare(&mut self, info: &egui_table::PrefetchInfo) {
        self.prefetched_offset = info.visible_rows.start as usize;
        self.prefetched.clear();
        self.ctx.shared.log_records.get_range(
            self.logger_ctx,
            info.visible_rows.start as usize..info.visible_rows.end as usize,
            &mut self.prefetched,
        );
    }

    fn row_top_offset(&self, _ctx: &egui::Context, _table_id: egui::Id, row_index: u64) -> f32 {
        self.extra_row_heights
            .range(0..row_index)
            .map(|(_, height)| height)
            .sum::<f32>()
            + row_index as f32 * self.default_row_height()
    }

    fn default_row_height(&self) -> f32 {
        ROW_SIZE
    }

    fn header_cell_ui(&mut self, ui: &mut Ui, cell: &egui_table::HeaderCellInfo) {
        let egui_table::HeaderCellInfo { col_range, .. } = cell;

        egui::Frame::none()
            .inner_margin(egui::Margin::symmetric(4.0, 0.0))
            .show(ui, |ui| match col_range.start {
                0 => {
                    ui.label("TIME");
                }
                1 => {
                    ui.label("LEVEL");
                }
                2 => {
                    ui.label("CONTEXT");
                }
                3 => {
                    ui.label("MESSAGE");
                }
                _ => unreachable!(),
            });
    }

    fn cell_ui(&mut self, ui: &mut Ui, cell: &egui_table::CellInfo) {
        let egui_table::CellInfo { row_nr, col_nr, .. } = *cell;

        if row_nr % 2 == 0 {
            ui.painter()
                .rect_filled(ui.max_rect(), 0.0, ui.visuals().faint_bg_color);
        }

        let Some(record) = &self
            .prefetched
            .get(row_nr as usize - self.prefetched_offset)
        else {
            return;
        };

        egui::Frame::none()
            .inner_margin(egui::Margin::symmetric(4.0, 4.0))
            .show(ui, |ui| match col_nr {
                0 => {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(
                                record.value.time().format("%F %H:%M:%S%.3f").to_string(),
                            )
                            .monospace()
                            .weak(),
                        );
                    });
                }
                1 => {
                    ui.vertical(|ui| {
                        let color = match record.value.static_data.level {
                            tinylog::Level::Trace => Color32::LIGHT_GRAY,
                            tinylog::Level::Debug => Color32::LIGHT_BLUE,
                            tinylog::Level::Info => Color32::LIGHT_GREEN,
                            tinylog::Level::Warn => Color32::LIGHT_YELLOW,
                            tinylog::Level::Error => Color32::LIGHT_RED,
                        };

                        ui.label(
                            RichText::new(record.value.static_data.level.to_string())
                                .monospace()
                                .color(color)
                                .strong(),
                        );
                    });
                }
                2 => {
                    ui.vertical(|ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                        ui.label(RichText::new(record.ctx.to_string()).monospace());
                    });
                }
                3 => {
                    ui.vertical(|ui| {
                        let frame_response = egui::Frame::none().show(ui, |ui| {
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                            ui.label(record.value.message.to_string());
                        });

                        let frame_size = frame_response.response.rect.size();
                        self.extra_row_heights
                            .insert(cell.row_nr, 8.0 + frame_size.y - ROW_SIZE);
                    });
                }
                _ => unreachable!(),
            });
    }
}

pub struct LogViewer {
    id: u64,
    row_heights: BTreeMap<u64, f32>,
    prefetch_buffer: Vec<RecordWithCtx>,

    // user settings
    logger_ctx: LoggerContext,
    logger_ctx_text: String,
    stick_to_bottom: bool,
}

impl LogViewer {
    fn draw_header(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.stick_to_bottom, "Stick to Bottom");

            ui.add_space(75.0);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.logger_ctx_text)
                        .hint_text("e.g. gpu for psx::gpu")
                        .desired_width(150.0),
                );

                if response.lost_focus() {
                    let segments = self
                        .logger_ctx_text
                        .split("::")
                        .filter(|seg| !seg.is_empty());
                    self.logger_ctx =
                        segments.fold(LoggerContext::new("psx"), |acc, seg| acc.child(seg));
                    self.stick_to_bottom = true;
                }
                ui.label("Filter: ");
            });
        });
    }

    fn draw_logs(&mut self, ui: &mut Ui, ctx: Context) {
        let logs = &ctx.shared.log_records;
        let logs_len = logs.len(self.logger_ctx.clone());

        ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();
        ui.style_mut().scroll_animation = ScrollAnimation::none();

        let message_width = (ui.available_width() - 180.0 - 50.0 - 100.0).max(100.0);
        let columns = [
            egui_table::Column::new(180.0).resizable(false),
            egui_table::Column::new(50.0).resizable(false),
            egui_table::Column::new(100.0).resizable(false),
            egui_table::Column::new(message_width).resizable(false),
        ];

        let id_salt = egui::Id::new(self.id);
        let mut table = egui_table::Table::new()
            .id_salt(id_salt)
            .num_rows(logs_len as u64)
            .columns(columns)
            .num_sticky_cols(0)
            .headers([egui_table::HeaderRow {
                height: ROW_SIZE,
                groups: Vec::new(),
            }])
            .auto_size_mode(egui_table::AutoSizeMode::OnParentResize);

        if self.stick_to_bottom {
            table = table.scroll_to_row(logs_len as u64, None);
        }

        table.show(
            ui,
            &mut LogTableDelegate {
                ctx,
                logger_ctx: &self.logger_ctx,
                extra_row_heights: &mut self.row_heights,
                prefetched: &mut self.prefetch_buffer,
                prefetched_offset: 0,
            },
        );
    }
}

impl Tab for LogViewer {
    fn new(id: u64) -> Self
    where
        Self: Sized,
    {
        Self {
            id,
            row_heights: BTreeMap::new(),
            prefetch_buffer: Vec::new(),

            logger_ctx: LoggerContext::new("psx"),
            logger_ctx_text: String::new(),
            stick_to_bottom: true,
        }
    }

    fn title(&mut self) -> egui::WidgetText {
        "Logs".into()
    }

    fn ui(&mut self, ui: &mut Ui, ctx: Context) {
        let response = egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.vertical(|ui| {
                self.draw_header(ui);
                ui.separator();
                self.draw_logs(ui, ctx);
            });
        });

        if response.response.hovered() {
            let scrolled = ui.input(|i| i.raw_scroll_delta.length_sq() > 0.0);
            if scrolled {
                self.stick_to_bottom = false;
            }
        }
    }

    fn multiple_allowed() -> bool
    where
        Self: Sized,
    {
        true
    }
}
