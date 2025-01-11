use crate::{
    colors::LIGHT_PURPLE,
    tab::{Context, Tab},
    util::character_dimensions,
};
use eframe::{
    egui::{self, RichText, Ui},
    epaint::Color32,
};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use shimmer_core::{
    cpu::instr::{Args, ImmKind, Instruction, RegSource},
    mem::Address,
};

fn ascii_score(bytes: impl Iterator<Item = u8>) -> u16 {
    let is_ascii_text = |value: u8| {
        value.is_ascii_alphanumeric() || value.is_ascii_punctuation() || value.is_ascii_whitespace()
    };

    let mut score: u16 = 0;
    let mut consecutive: u16 = 0;
    let mut last_was_null = false;
    for byte in bytes {
        if is_ascii_text(byte) {
            consecutive += 1;
        } else if byte == 0 && !last_was_null {
            consecutive += 1;
            last_was_null = true;
        } else {
            score += consecutive.saturating_sub(1);
            consecutive = 0;
            last_was_null = false;
        }
    }

    score + consecutive.saturating_sub(1)
}

pub struct InstructionViewer {
    target: u32,
    target_text: String,
    target_view: u32,
    follow_next: bool,
    old_next: u32,
    commonmark_cache: CommonMarkCache,
}

impl InstructionViewer {
    fn draw_header(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let (font_width, _) = character_dimensions(ui, egui::TextStyle::Monospace, 'A');

            ui.checkbox(&mut self.follow_next, "Follow Next");
            if !self.follow_next {
                ui.label("Address:");
                let target_response = ui.add(
                    egui::TextEdit::singleline(&mut self.target_text)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(8.5 * font_width),
                );

                if target_response.changed() {
                    self.target_text.retain(|c| c.is_ascii_hexdigit());
                    self.target = u32::from_str_radix(&self.target_text, 16).unwrap_or(0) & !0b11;
                    self.target_view = self.target;
                }

                if target_response.lost_focus() {
                    self.target_text = format!("{:08X}", self.target);
                }
            }
        });
    }

    fn draw_args(&mut self, ui: &mut Ui, alternative_names: bool, instr: Instruction, args: Args) {
        const RD_COLOR: Color32 = Color32::LIGHT_GREEN;
        const RS_COLOR: Color32 = Color32::LIGHT_RED;
        const RT_COLOR: Color32 = Color32::LIGHT_BLUE;
        const IMM_COLOR: Color32 = LIGHT_PURPLE;

        macro_rules! helper {
            ($reg:ident, $color:ident, $call:ident) => {
                if let Some(src) = args.$reg {
                    match src {
                        RegSource::CPU => {
                            let reg = instr.$call();
                            let name = if alternative_names {
                                RichText::new(reg.alt_name())
                            } else {
                                RichText::new(format!("{reg:?}"))
                            };

                            let response = ui.label(name.color($color).monospace());
                            response.on_hover_text(stringify!($reg));
                        }
                        RegSource::COP0 => {
                            let reg = instr.$call();
                            let name = RichText::new(format!("COP0_{reg:?}"));
                            let response = ui.label(name.color($color).monospace());
                            response.on_hover_text(stringify!($reg));
                        }
                        RegSource::COP1 => {
                            let reg = instr.$call();
                            let name = RichText::new(format!("COP1_{reg:?}"));
                            let response = ui.label(name.color($color).monospace());
                            response.on_hover_text(stringify!($reg));
                        }
                        RegSource::COP2 => {
                            let reg = instr.$call();
                            let name = RichText::new(format!("COP2_{reg:?}"));
                            let response = ui.label(name.color($color).monospace());
                            response.on_hover_text(stringify!($reg));
                        }
                        RegSource::COP3 => {
                            let reg = instr.$call();
                            let name = RichText::new(format!("COP3_{reg:?}"));
                            let response = ui.label(name.color($color).monospace());
                            response.on_hover_text(stringify!($reg));
                        }
                    }
                }
            };
        }

        helper!(rd, RD_COLOR, rd);
        helper!(rt, RT_COLOR, rt);
        helper!(rs, RS_COLOR, rs);

        if let Some(imm_kind) = args.imm {
            let (imm, width) = match imm_kind {
                ImmKind::U5 => (instr.imm5().value() as u32, 5usize),
                ImmKind::U16 => (instr.imm16() as u32, 16),
                ImmKind::I16 => (instr.imm16() as u32, 16),
                ImmKind::U20 => (instr.imm20().value(), 20),
                ImmKind::U26 => (instr.imm26().value(), 26),
            };

            let hex_width = (width.div_ceil(4)).next_multiple_of(2);
            let response = ui.label(
                RichText::new(format!("0x{imm:0hex_width$X}"))
                    .color(IMM_COLOR)
                    .monospace(),
            );
            response.on_hover_text("immediate value");
        }
    }

    fn draw_row(&mut self, ui: &mut Ui, ctx: &Context, addr: u32, instr: Instruction, valid: bool) {
        const MNEMONIC_COLOR: Color32 = Color32::LIGHT_YELLOW;

        let mnemonic = instr.mnemonic().unwrap_or_else(|| "ILLEGAL".into());
        let description = "TODO";
        let args = instr.args().unwrap_or_default();

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{:08X}", addr))
                    .color(if addr == self.target {
                        Color32::LIGHT_RED
                    } else if valid {
                        Color32::LIGHT_BLUE
                    } else {
                        Color32::DARK_GRAY
                    })
                    .monospace(),
            );
            let mnemonic_response =
                ui.label(RichText::new(mnemonic).color(MNEMONIC_COLOR).monospace());
            mnemonic_response.on_hover_ui(|ui| {
                CommonMarkViewer::new().show(ui, &mut self.commonmark_cache, description);
            });
        });

        ui.horizontal(|ui| {
            self.draw_args(ui, ctx.exclusive.controls.alternative_names, instr, args);
        });
    }

    fn draw_body(&mut self, ui: &mut Ui, ctx: &mut Context) {
        let (_, available_height) = ui.available_size().into();
        let (_, font_height) = character_dimensions(ui, egui::TextStyle::Monospace, 'A');

        let row_height = font_height;
        let row_count = (available_height / row_height) as usize;
        let begin_addr = self
            .target_view
            .saturating_sub((4 * (row_count & !1) / 2) as u32);

        ui.spacing_mut().item_spacing.y = -4.1;
        egui::Grid::new("instr_grid")
            .min_col_width(125.0)
            .show(ui, |ui| {
                for row in 0..row_count {
                    let addr = begin_addr + row as u32 * 4;

                    let prev_instr = ctx
                        .exclusive
                        .psx
                        .psx_mut()
                        .read_unaligned::<u32, true>(Address(addr.saturating_sub(4)));
                    let instr = ctx
                        .exclusive
                        .psx
                        .psx_mut()
                        .read_unaligned::<u32, true>(Address(addr));
                    let next_instr = ctx
                        .exclusive
                        .psx
                        .psx_mut()
                        .read_unaligned::<u32, true>(Address(addr.saturating_add(4)));

                    // heuristic to determine if it is likely to be a real instruction or not
                    let bytes = prev_instr
                        .to_le_bytes()
                        .into_iter()
                        .chain(instr.to_le_bytes())
                        .chain(next_instr.to_le_bytes());

                    let invalid_score = || {
                        let ascii_score = ascii_score(bytes);
                        let illegal_score = match (
                            Instruction::from_bits(prev_instr).is_illegal(),
                            Instruction::from_bits(next_instr).is_illegal(),
                        ) {
                            (true, true) => 6,
                            (false, false) => -1,
                            _ => 2,
                        };

                        ascii_score.saturating_add_signed(illegal_score)
                    };

                    let valid = !Instruction::from_bits(instr).is_illegal() && invalid_score() <= 5;
                    self.draw_row(ui, ctx, addr, Instruction::from_bits(instr), valid);
                    ui.end_row();
                }
            });

        if ctx.is_focused {
            let scrolled = ui.input(|i| i.smooth_scroll_delta);
            self.target_view = self
                .target_view
                .saturating_add_signed((-scrolled.y / 8.0) as i32 * 4);
        }
    }
}

impl Tab for InstructionViewer {
    fn new(_: u64) -> Self {
        Self {
            target: 0xBFC0_0000,
            target_view: 0xBFC0_0000,
            target_text: String::from("BFC00000"),
            follow_next: true,
            old_next: 0,
            commonmark_cache: CommonMarkCache::default(),
        }
    }

    fn multiple_allowed() -> bool {
        true
    }

    fn title(&mut self) -> egui::WidgetText {
        "Instruction Viewer".into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, mut ctx: Context) {
        let next = ctx.exclusive.psx.psx().cpu.instr_delay_slot().1;
        if self.follow_next && next != self.old_next {
            self.target = next.value();
            self.target_view = next.value();
            self.old_next = next.value();
        }

        ui.vertical(|ui| {
            self.draw_header(ui);
            ui.separator();
            self.draw_body(ui, &mut ctx);
        });
    }
}
