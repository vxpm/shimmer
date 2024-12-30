use crate::{
    colors::BG_HIGHLIGHT_LIGHTER,
    tab::{Context, Tab},
    util::character_dimensions,
};
use eframe::{
    egui::{self, RichText, Ui},
    epaint::Color32,
};
use shimmer_core::mem::Address;
use strum::{AsRefStr, VariantArray};

struct Widths {
    // available: f32,

    // address
    // addr: f32,
    addr_hex_bytes_divisor: f32,
    // available_after_addr: f32,

    // hex bytes
    // hex_byte: f32,
    hex_bytes_divisor: f32,
    hex_bytes_ascii_bytes_divisor: f32,

    // ascii bytes
    // ascii_byte: f32,
    ascii_bytes_divisor: f32,

    // bytes
    // byte: f32,
    byte_count: usize,
}

impl Widths {
    pub fn new(available_width: f32, font_width: f32) -> Self {
        // address
        let addr = 8.0 * font_width;
        let addr_hex_bytes_divisor = 8.0;
        let available_after_addr = available_width - addr - addr_hex_bytes_divisor;

        // hex bytes
        let hex_byte = 2.0 * font_width;
        let hex_bytes_divisor = 2.0;
        let hex_bytes_ascii_bytes_divisor = 8.0;

        // ascii bytes
        let ascii_byte = font_width;
        let ascii_bytes_divisor = 1.5;

        let byte = hex_byte
            + hex_bytes_divisor
            + 0.2 // to workaround imprecisions
            + ascii_byte
            + ascii_bytes_divisor
            + 0.2; // same reason

        let byte_count = (available_after_addr / byte) as usize;

        Self {
            // available: available_width,
            // addr,
            addr_hex_bytes_divisor,
            // available_after_addr,
            // hex_byte,
            hex_bytes_divisor,
            hex_bytes_ascii_bytes_divisor,
            // ascii_byte,
            ascii_bytes_divisor,
            // byte,
            byte_count,
        }
    }
}

fn char_to_symbol(c: char) -> char {
    if !c.is_ascii() {
        '⬚'
    } else if c.is_ascii_control() {
        match c {
            '\0' => '∘',
            '\n' => '↲',
            _ => '∗',
        }
    } else if c == ' ' {
        '‧'
    } else {
        c
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRefStr, VariantArray)]
enum Visualization {
    U8,
    U16,
    U32,
    I8,
    I16,
    I32,
    F32,
}

pub struct MemoryViewer {
    target: u32,
    target_text: String,
    visualization: Visualization,
    view_target: u32,
    mem_values: Vec<Option<u8>>,
}

impl MemoryViewer {
    fn draw_header(&mut self, ui: &mut Ui, ctx: &mut Context) {
        ui.horizontal(|ui| {
            let (font_width, _) = character_dimensions(ui, egui::TextStyle::Monospace, 'A');

            ui.label("Address:");
            let target_response = ui.add(
                egui::TextEdit::singleline(&mut self.target_text)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(8.5 * font_width),
            );

            if target_response.changed() {
                self.target_text.retain(|c| c.is_ascii_hexdigit());
                self.target = u32::from_str_radix(&self.target_text, 16).unwrap_or(0);
                self.view_target = self.target;
            }

            if target_response.lost_focus() {
                self.target_text = format!("{:08X}", self.target);
            }

            ui.label("View:");
            egui::ComboBox::new("visualization", "")
                .selected_text(self.visualization.as_ref())
                .show_ui(ui, |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                    for variant in Visualization::VARIANTS {
                        ui.selectable_value(&mut self.visualization, *variant, variant.as_ref());
                    }
                });
        });

        let addr = Address(self.target);
        let view = match self.visualization {
            Visualization::U8 => ctx.shared.psx.bus().read_unaligned::<u8>(addr).to_string(),
            Visualization::U16 => ctx.shared.psx.bus().read_unaligned::<u16>(addr).to_string(),
            Visualization::U32 => ctx.shared.psx.bus().read_unaligned::<u32>(addr).to_string(),
            Visualization::I8 => ctx.shared.psx.bus().read_unaligned::<i8>(addr).to_string(),
            Visualization::I16 => ctx.shared.psx.bus().read_unaligned::<i16>(addr).to_string(),
            Visualization::I32 => ctx.shared.psx.bus().read_unaligned::<i32>(addr).to_string(),
            Visualization::F32 => (unsafe {
                std::mem::transmute::<u32, f32>(ctx.shared.psx.bus().read_unaligned::<u32>(addr))
            })
            .to_string(),
        };

        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
        ui.label(egui::RichText::new(view));
    }

    fn draw_row_hex(&mut self, ui: &mut Ui, widths: &Widths, base: u32) {
        for offset in 0..widths.byte_count {
            if let Some(value) = self.mem_values[offset] {
                let addr = base as usize + offset;

                let color = colorous::COOL.eval_rational(value as usize, 256usize);
                let bg_color = if addr as u32 == self.target {
                    Color32::LIGHT_RED
                } else if (addr >> 2) & 1 == 0 {
                    ui.style().visuals.window_fill
                } else {
                    BG_HIGHLIGHT_LIGHTER
                };

                let label_response = ui.label(
                    RichText::new(format!("{:02X}", value))
                        .monospace()
                        .color(Color32::from_rgb(color.r, color.g, color.b))
                        .background_color(bg_color),
                );

                if label_response.clicked() {
                    self.target = addr as u32;
                    self.target_text = format!("{:08X}", self.target);
                }

                ui.add_space(widths.hex_bytes_divisor);
            } else {
                ui.label(RichText::new("  ").monospace());
                ui.add_space(widths.hex_bytes_divisor);
            }
        }
    }

    fn draw_row_ascii(&mut self, ui: &mut Ui, widths: &Widths, base: u32) {
        for offset in 0..widths.byte_count {
            if let Some(value) = self.mem_values[offset] {
                let addr = base as usize + offset;
                let char = value as char;

                let color = if char.is_ascii_alphanumeric()
                    || char.is_ascii_punctuation()
                    || char.is_ascii_whitespace()
                {
                    Color32::LIGHT_BLUE
                } else {
                    Color32::DARK_GRAY
                };
                let bg_color = if addr as u32 == self.target {
                    BG_HIGHLIGHT_LIGHTER
                } else {
                    ui.style().visuals.window_fill
                };

                let char_symbol = char_to_symbol(char);
                let label_response = ui.label(
                    RichText::new(char_symbol)
                        .text_style(egui::TextStyle::Monospace)
                        .color(color)
                        .background_color(bg_color),
                );

                if label_response.clicked() {
                    self.target = addr as u32;
                    self.target_text = format!("{:08X}", self.target);
                }

                ui.add_space(widths.ascii_bytes_divisor);
            } else {
                ui.label(RichText::new("").monospace());
                ui.add_space(widths.ascii_bytes_divisor);
            }
        }
    }

    fn draw_row(&mut self, ui: &mut Ui, widths: &Widths, base: u32) {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{:08X}", base))
                    .text_style(egui::TextStyle::Monospace)
                    .color(Color32::LIGHT_BLUE),
            );
            ui.add_space(widths.addr_hex_bytes_divisor);
            self.draw_row_hex(ui, widths, base);
            ui.add_space(widths.hex_bytes_ascii_bytes_divisor - widths.hex_bytes_divisor);
            self.draw_row_ascii(ui, widths, base);
        });
    }

    fn draw_body(&mut self, ui: &mut Ui, ctx: &mut Context) -> Widths {
        let (available_width, available_height) = ui.available_size().into();
        let (font_width, font_height) = character_dimensions(ui, egui::TextStyle::Monospace, 'A');

        let widths = Widths::new(available_width, font_width);
        let row_height = font_height;
        let row_count = (available_height / row_height) as usize;
        let target_base = self.view_target - self.view_target % widths.byte_count as u32;
        let begin_base =
            target_base.saturating_sub((widths.byte_count * (row_count & !1) / 2) as u32);

        ui.spacing_mut().item_spacing.y = -4.1;
        ui.vertical(|ui| {
            for row in 0..row_count {
                let base = begin_base + row as u32 * widths.byte_count as u32;
                self.mem_values.clear();
                self.mem_values.extend((0..widths.byte_count).map(|offset| {
                    base.checked_add(offset as u32)
                        .map(|addr| ctx.shared.psx.bus().read_unaligned::<u8>(Address(addr)))
                }));

                self.draw_row(ui, &widths, base as u32);
            }
        });

        widths
    }
}

impl Tab for MemoryViewer {
    fn new(_: u64) -> Self
    where
        Self: Sized,
    {
        Self {
            target: 0xBFC0_0000,
            target_text: String::from("BFC00000"),
            visualization: Visualization::U32,
            view_target: 0xBFC0_0000,
            mem_values: Vec::new(),
        }
    }

    fn title(&mut self) -> egui::WidgetText {
        "Memory Viewer".into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, mut ctx: Context) {
        let mut widths = None;
        let response = egui::panel::CentralPanel::default().show_inside(ui, |ui| {
            ui.vertical(|ui| {
                self.draw_header(ui, &mut ctx);
                ui.separator();
                widths = Some(self.draw_body(ui, &mut ctx));
            });
        });

        if response.response.hovered() {
            let scrolled = ui.input(|i| i.smooth_scroll_delta);
            self.view_target = self.view_target.saturating_add_signed(
                (-scrolled.y / 8.0) as i32 * widths.unwrap().byte_count as i32,
            );
        }
    }
}
