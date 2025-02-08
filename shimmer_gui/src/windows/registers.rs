use super::WindowUi;
use crate::ExclusiveState;
use eframe::egui::{self, Color32, Id, RichText, Ui, Vec2, Window};
use egui_taffy::{
    TuiBuilderLogic,
    taffy::{
        self,
        prelude::{auto, length, percent},
    },
    tui,
};
use shimmer_core::cpu::Reg;
use strum::VariantArray;

pub struct Registers {
    id: Id,
}

impl Registers {
    pub fn new(id: Id) -> Self
    where
        Self: Sized,
    {
        Self { id }
    }
}

impl WindowUi for Registers {
    fn build<'open>(&mut self, open: &'open mut bool) -> Window<'open> {
        Window::new("Registers")
            .open(open)
            .min_width(200.0)
            .max_height(0.0)
            .default_size(Vec2::new(0.0, 0.0))
    }

    fn show(&mut self, state: &mut ExclusiveState, ui: &mut Ui) {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        let default_style = || taffy::Style {
            padding: length(4.),
            gap: length(4.),
            flex_grow: 1.,
            justify_content: Some(taffy::AlignContent::Center),
            ..Default::default()
        };

        tui(ui, self.id)
            .reserve_available_space()
            .style(taffy::Style {
                flex_direction: taffy::FlexDirection::Column,
                align_items: Some(taffy::AlignItems::Stretch),
                size: taffy::Size {
                    width: percent(1.),
                    height: auto(),
                },
                ..default_style()
            })
            .show(|tui| {
                tui.style(taffy::Style {
                    flex_wrap: taffy::FlexWrap::Wrap,
                    justify_items: Some(taffy::AlignItems::Stretch),
                    ..default_style()
                })
                .add(|tui| {
                    for reg in Reg::VARIANTS {
                        tui.style(default_style()).add_with_border(|tui| {
                            let value = state.emulator.psx().cpu.regs().read(*reg);
                            let name = if state.controls.alternative_names {
                                RichText::new(reg.alt_name())
                            } else {
                                RichText::new(format!("{:?}", reg))
                            };
                            let description = reg.description();

                            let response = tui.label(name.monospace().color(Color32::LIGHT_BLUE));
                            response.on_hover_ui(|ui| {
                                ui.label(description);
                            });

                            tui.label(
                                RichText::new(format!("{:08X}", value))
                                    .monospace()
                                    .color(Color32::LIGHT_GREEN),
                            );
                        });
                    }
                });
            });
    }
}
