use crate::tab::{Context, Tab};
use eframe::egui::{self, Ui};
use egui_virtual_list::VirtualList;
use strum::{AsRefStr, VariantArray};
use tinylog::Level;

#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRefStr, VariantArray)]
enum Source {
    CPU,
    Memory,
    COP0,
    GPU,
}

pub struct LogViewer {
    id: u64,
    src: Source,
    virtual_list: VirtualList,
}

impl LogViewer {
    fn draw_header(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Source:");
            egui::ComboBox::from_label("")
                .selected_text(self.src.as_ref())
                .show_ui(ui, |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                    ui.set_min_width(60.0);
                    for variant in Source::VARIANTS {
                        if ui
                            .selectable_value(&mut self.src, *variant, variant.as_ref())
                            .changed()
                        {
                            self.virtual_list.reset();
                        }
                    }
                });
        });
    }

    fn draw_logs(&mut self, ui: &mut Ui, ctx: Context) {
        let logs = &ctx.shared.log_records;

        let log_ctx = tinylog::logger::Context::new("psx");
        let logs_len = logs.len(log_ctx.clone());

        self.virtual_list.hide_on_resize(None);
        ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();

        // create a scrollable area for the logs
        egui::ScrollArea::both()
            .auto_shrink(false)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

                let mut logs_buf = Vec::new();
                let mut id = 0;
                self.virtual_list
                    .ui_custom_layout(ui, logs_len, |ui, start_index| {
                        egui::Grid::new(id).min_col_width(40.0).show(ui, |ui| {
                            id += 1;
                            logs.get_range(
                                log_ctx.clone(),
                                start_index..start_index + 1,
                                &mut logs_buf,
                            );

                            let log = &logs_buf[0];
                            let time = egui::RichText::new(format!(
                                "{}",
                                log.value
                                    .time()
                                    // .with_timezone(&chrono::Local)
                                    .format("%H:%M:%S.%3f")
                            ))
                            .color(egui::Color32::GRAY)
                            .text_style(egui::TextStyle::Monospace);

                            let level = egui::RichText::new(
                                log.value.static_data.level.to_string(),
                            )
                            .color(match log.value.static_data.level {
                                Level::Trace => egui::Color32::GRAY,
                                Level::Debug => egui::Color32::GRAY,
                                Level::Info => egui::Color32::LIGHT_BLUE,
                                Level::Warn => egui::Color32::LIGHT_YELLOW,
                                Level::Error => egui::Color32::LIGHT_RED,
                            });

                            let log_msg = log.value.message.to_string();
                            let mut lines = log_msg.lines();
                            let first_line = lines.next().unwrap_or("...");
                            let has_more = lines.next().is_some();

                            let message = egui::RichText::new(first_line)
                                .color(if has_more {
                                    egui::Color32::GRAY
                                } else {
                                    egui::Color32::LIGHT_GRAY
                                })
                                .monospace();

                            ui.label(time);
                            ui.label(level);
                            ui.horizontal(|ui| {
                                ui.separator();

                                let response = ui.label(message);
                                if has_more {
                                    response.on_hover_text(
                                        egui::RichText::new(log_msg.as_str())
                                            .color(egui::Color32::LIGHT_GRAY)
                                            // .small(),
                                            .monospace(),
                                    );
                                }
                            });

                            ui.end_row();
                        });

                        1
                    });
            });
    }
}

impl Tab for LogViewer {
    fn new(id: u64) -> Self
    where
        Self: Sized,
    {
        let mut list = VirtualList::new();
        list.hide_on_resize(None);

        Self {
            id,
            src: Source::CPU,
            virtual_list: list,
        }
    }

    fn title(&mut self) -> egui::WidgetText {
        "Logs".into()
    }

    fn ui(&mut self, ui: &mut Ui, ctx: Context) {
        ui.vertical(|ui| {
            self.draw_header(ui);
            ui.separator();
            self.draw_logs(ui, ctx);
        });
    }

    fn multiple_allowed() -> bool
    where
        Self: Sized,
    {
        true
    }
}
