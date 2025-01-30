use crate::tab::{Context, Tab};
use eframe::egui::{self, Color32, RichText, Ui, UiBuilder, Vec2, style::ScrollAnimation};
use egui_table::TableDelegate;
use std::{cell::RefCell, collections::BTreeMap};
use tinylog::{Level, logger::Context as LoggerContext, record::RecordWithCtx};

const ROW_SIZE: f32 = 30.0;

struct LogTableDelegate<'a> {
    ctx: Context<'a>,
    logger_ctx: &'a LoggerContext,
    message_width: f32,
    row_heights: &'a mut BTreeMap<u64, f32>,
    row_top_offsets: &'a mut RefCell<Vec<Option<f32>>>,
    prefetched: &'a mut BTreeMap<u64, RecordWithCtx>,
}

impl TableDelegate for LogTableDelegate<'_> {
    fn prepare(&mut self, info: &egui_table::PrefetchInfo) {
        self.prefetched.clear();

        let mut buf = Vec::with_capacity(32);
        self.ctx.exclusive.log_records.get_range(
            self.logger_ctx,
            info.visible_rows.start as usize..info.visible_rows.end as usize,
            &mut buf,
        );

        for (row_nr, record) in info.visible_rows.clone().zip(buf) {
            self.prefetched.insert(row_nr, record);
        }
    }

    fn row_top_offset(&self, _ctx: &egui::Context, _table_id: egui::Id, row_nr: u64) -> f32 {
        let mut row_top_offsets = self.row_top_offsets.borrow_mut();
        if row_top_offsets.len() <= row_nr as usize {
            row_top_offsets.resize(row_nr as usize + 1, None);
        }

        match row_top_offsets[row_nr as usize] {
            Some(offset) => offset,
            None => {
                let mut known = 0;
                let known_height = self
                    .row_heights
                    .range(0..row_nr)
                    .map(|r| {
                        known += 1;
                        *r.1
                    })
                    .sum::<f32>();

                let missing = row_nr - known;
                let top_offset = known_height + ROW_SIZE * missing as f32;
                row_top_offsets[row_nr as usize] = Some(top_offset);

                top_offset
            }
        }
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

        let Some(record) = &self.prefetched.get(&row_nr) else {
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
                        let (_, space) = ui.allocate_space(Vec2::new(self.message_width, ROW_SIZE));
                        let response = ui.scope_builder(UiBuilder::new().max_rect(space), |ui| {
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                            ui.label(record.value.message.to_string());

                            if !record.value.attachments.is_empty() {
                                for attachment in &record.value.attachments {
                                    let key = &attachment.key;
                                    let value = if let Some(display) = attachment.value.as_display()
                                    {
                                        display.to_string()
                                    } else if let Some(debug) = attachment.value.as_debug() {
                                        format!("{debug:?}")
                                    } else {
                                        "(opaque)".to_string()
                                    };

                                    ui.label(
                                        RichText::new(format!("{key:?}: {}", value)).small().weak(),
                                    );
                                }
                            }
                        });

                        let size = 8.0 + response.response.rect.size().y;
                        let old = self.row_heights.insert(cell.row_nr, size);

                        if old != Some(size) {
                            let mut row_top_offsets = self.row_top_offsets.borrow_mut();
                            row_top_offsets[cell.row_nr as usize..]
                                .iter_mut()
                                .for_each(|o| *o = None);
                        }
                    });
                }
                _ => unreachable!(),
            });
    }
}

pub struct LogViewer {
    id: u64,
    row_heights: BTreeMap<u64, f32>,
    row_top_offsets: RefCell<Vec<Option<f32>>>,
    prefetch_buffer: BTreeMap<u64, RecordWithCtx>,

    // user settings
    logger_ctx: LoggerContext,
    stick_to_bottom: bool,
}

impl LogViewer {
    fn draw_header(&mut self, ui: &mut Ui, ctx: &mut Context) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.stick_to_bottom, "Stick to Bottom");

            ui.add_space(75.0);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let current_level = ctx.exclusive.log_family.level_of(&self.logger_ctx).unwrap();
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
                    ctx.exclusive
                        .log_family
                        .set_level_of(&self.logger_ctx, new_level)
                        .unwrap();
                }

                let response = egui::ComboBox::from_label("Context:")
                    .selected_text(self.logger_ctx.to_string())
                    .show_ui(ui, |ui| {
                        let mut changed = false;
                        for context in ctx.exclusive.log_family.contexts() {
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
                    self.row_top_offsets.borrow_mut().clear();
                }
            });
        });
    }

    fn draw_logs(&mut self, ui: &mut Ui, ctx: Context) {
        let logs = &ctx.exclusive.log_records;
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
            .auto_size_mode(egui_table::AutoSizeMode::Never);

        if self.stick_to_bottom {
            table = table.scroll_to_row(logs_len as u64, None);
        }

        table.show(ui, &mut LogTableDelegate {
            ctx,
            logger_ctx: &self.logger_ctx,
            message_width,
            row_heights: &mut self.row_heights,
            row_top_offsets: &mut self.row_top_offsets,
            prefetched: &mut self.prefetch_buffer,
        });
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
            row_top_offsets: RefCell::new(Vec::new()),
            prefetch_buffer: BTreeMap::new(),

            logger_ctx: LoggerContext::new("psx"),
            stick_to_bottom: true,
        }
    }

    fn title(&mut self) -> egui::WidgetText {
        "Logs".into()
    }

    fn ui(&mut self, ui: &mut Ui, mut ctx: Context) {
        let response = egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.vertical(|ui| {
                self.draw_header(ui, &mut ctx);
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
