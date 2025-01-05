use crate::{
    tab::{Context, Tab},
    util::character_dimensions,
};
use eframe::{
    egui::{self, RichText, Ui},
    epaint::Color32,
};

pub struct Breakpoints {
    target_text: String,
}

impl Breakpoints {
    pub fn draw_header(&mut self, ui: &mut Ui, ctx: &mut Context) {
        let (font_width, _) = character_dimensions(ui, egui::TextStyle::Monospace, 'A');

        ui.horizontal(|ui| {
            ui.label("Address:");
            let target_response = ui.add(
                egui::TextEdit::singleline(&mut self.target_text)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(8.5 * font_width),
            );

            if target_response.changed() {
                self.target_text.retain(|c| c.is_ascii_hexdigit());
            }

            if target_response.lost_focus() {
                let target = u32::from_str_radix(&self.target_text, 16).unwrap_or(0) & !0b11;
                self.target_text = format!("{:08X}", target);
            }

            if ui.button("Add").clicked() {
                let target = u32::from_str_radix(&self.target_text, 16).unwrap_or(0) & !0b11;

                if !ctx.exclusive.controls.breakpoints.contains(&target) {
                    ctx.exclusive.controls.breakpoints.push(target);
                }
            }
        });
    }

    pub fn draw_body(&mut self, ui: &mut Ui, ctx: &mut Context) {
        let (_, font_height) = character_dimensions(ui, egui::TextStyle::Monospace, 'A');

        let mut to_remove = Vec::new();
        egui::ScrollArea::vertical().show_rows(
            ui,
            font_height,
            ctx.exclusive.controls.breakpoints.len(),
            |ui, row_range| {
                for &breakpoint in &ctx.exclusive.controls.breakpoints[row_range] {
                    ui.horizontal(|ui| {
                        let color =
                            if ctx.exclusive.psx.bus().cpu.instr_delay_slot().1 == breakpoint {
                                Color32::LIGHT_RED
                            } else {
                                Color32::LIGHT_BLUE
                            };
                        ui.label(
                            RichText::new(format!("0x{:08X}", breakpoint))
                                .monospace()
                                .color(color),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("🗑").clicked() {
                                to_remove.push(breakpoint);
                            }
                        });
                    });
                }
            },
        );

        ctx.exclusive
            .controls
            .breakpoints
            .retain(|b| !to_remove.contains(b));
    }
}

impl Tab for Breakpoints {
    fn new(_: u64) -> Self
    where
        Self: Sized,
    {
        Self {
            target_text: String::from("00000000"),
        }
    }

    fn title(&mut self) -> egui::WidgetText {
        "Breakpoints".into()
    }

    fn ui(&mut self, ui: &mut Ui, mut ctx: Context) {
        ui.vertical(|ui| {
            self.draw_header(ui, &mut ctx);
            ui.separator();
            self.draw_body(ui, &mut ctx);
        });
    }
}
