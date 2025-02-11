use super::WindowUi;
use crate::ExclusiveState;
use eframe::{
    egui::{self, Id, RichText, Ui, Window},
    epaint::Color32,
};
use egui_extras::{Column, TableBuilder, TableRow};
use shimmer::core::{
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

fn fetch_instr(state: &mut ExclusiveState, addr: Address) -> (Instruction, bool) {
    let prev_instr = state
        .emulator
        .psx_mut()
        .read_unaligned::<u32, true>(Address(addr.value().saturating_sub(4)));
    let instr = state.emulator.psx_mut().read_unaligned::<u32, true>(addr);
    let next_instr = state
        .emulator
        .psx_mut()
        .read_unaligned::<u32, true>(Address(addr.value().saturating_add(4)));

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

    let instr = Instruction::from_bits(instr);
    let valid = !instr.is_illegal() && invalid_score() <= 5;
    (instr, valid)
}

pub struct InstructionViewer {
    target: u32,
    target_text: String,
    follow_next: bool,
}

impl InstructionViewer {
    fn draw_header(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.follow_next, "Follow Next");
            if !self.follow_next {
                ui.label("Address:");
                let target_response = ui.add(
                    egui::TextEdit::singleline(&mut self.target_text)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(85.0),
                );

                if target_response.changed() {
                    self.target_text.retain(|c| c.is_ascii_hexdigit());
                    self.target = u32::from_str_radix(&self.target_text, 16).unwrap_or(0) & !0b11;
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
        const IMM_COLOR: Color32 = Color32::LIGHT_GRAY;

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

    fn draw_row(&mut self, state: &mut ExclusiveState, row: &mut TableRow, begin_addr: u32) {
        const MNEMONIC_COLOR: Color32 = Color32::LIGHT_YELLOW;

        let address = Address(begin_addr + row.index() as u32 * 4);
        let (instr, valid) = fetch_instr(state, address);

        let mnemonic = instr.mnemonic().unwrap_or_else(|| "ILLEGAL".into());
        let description = "TODO";
        let args = instr.args().unwrap_or_default();

        row.col(|ui| {
            ui.label(
                RichText::new(format!("{:08X}", address.value()))
                    .color(if address == self.target {
                        Color32::LIGHT_RED
                    } else if valid {
                        Color32::LIGHT_BLUE
                    } else {
                        Color32::DARK_GRAY
                    })
                    .monospace(),
            );
        });

        row.col(|ui| {
            ui.horizontal(|ui| {
                let mnemonic_response = ui.label(
                    RichText::new(format!("{:8}", mnemonic))
                        .color(MNEMONIC_COLOR)
                        .monospace(),
                );
                mnemonic_response.on_hover_ui(|ui| {
                    ui.label(description);
                });

                self.draw_args(ui, state.controls.alternative_names, instr, args);
            });
        });
    }

    fn draw_body(&mut self, state: &mut ExclusiveState, ui: &mut Ui) {
        let count = 1024;
        let begin_addr = self.target.saturating_sub((4 * (count & !1) / 2) as u32);

        let builder = TableBuilder::new(ui)
            .auto_shrink([false; 2])
            .striped(true)
            .column(Column::auto().at_least(90.0))
            .column(Column::remainder());

        let builder = if self.follow_next {
            builder.scroll_to_row(512, Some(egui::Align::Center))
        } else {
            builder
        };

        builder
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label("Address");
                });

                header.col(|ui| {
                    ui.label("Instruction");
                });
            })
            .body(|body| {
                body.rows(20.0, count, |mut row| {
                    self.draw_row(state, &mut row, begin_addr);
                });
            });

        // egui::Grid::new("instr_grid")
        //     .min_col_width(125.0)
        //     .show(ui, |ui| {
        //         for row in 0..row_count {
        //             let addr = begin_addr + row as u32 * 4;
        //
        //             let prev_instr = state
        //                 .emulator
        //                 .psx_mut()
        //                 .read_unaligned::<u32, true>(Address(addr.saturating_sub(4)));
        //             let instr = state
        //                 .emulator
        //                 .psx_mut()
        //                 .read_unaligned::<u32, true>(Address(addr));
        //             let next_instr = state
        //                 .emulator
        //                 .psx_mut()
        //                 .read_unaligned::<u32, true>(Address(addr.saturating_add(4)));
        //
        //             // heuristic to determine if it is likely to be a real instruction or not
        //             let bytes = prev_instr
        //                 .to_le_bytes()
        //                 .into_iter()
        //                 .chain(instr.to_le_bytes())
        //                 .chain(next_instr.to_le_bytes());
        //
        //             let invalid_score = || {
        //                 let ascii_score = ascii_score(bytes);
        //                 let illegal_score = match (
        //                     Instruction::from_bits(prev_instr).is_illegal(),
        //                     Instruction::from_bits(next_instr).is_illegal(),
        //                 ) {
        //                     (true, true) => 6,
        //                     (false, false) => -1,
        //                     _ => 2,
        //                 };
        //
        //                 ascii_score.saturating_add_signed(illegal_score)
        //             };
        //
        //             let valid = !Instruction::from_bits(instr).is_illegal() && invalid_score() <= 5;
        //             self.draw_row(state, ui, addr, Instruction::from_bits(instr), valid);
        //             ui.end_row();
        //         }
        //     });
    }
}

impl InstructionViewer {
    pub fn new(_id: Id) -> Self {
        Self {
            target: 0xBFC0_0000,
            target_text: String::from("BFC00000"),
            follow_next: true,
        }
    }
}

impl WindowUi for InstructionViewer {
    fn build<'open>(&mut self, open: &'open mut bool) -> egui::Window<'open> {
        Window::new("Instructions").open(open)
    }

    fn show(&mut self, state: &mut ExclusiveState, ui: &mut Ui) {
        let next = state.emulator.psx().cpu.instr_delay_slot.1;
        if self.follow_next {
            self.target = next.value();
        }

        ui.vertical(|ui| {
            self.draw_header(ui);
            ui.separator();
            self.draw_body(state, ui);
        });
    }
}
