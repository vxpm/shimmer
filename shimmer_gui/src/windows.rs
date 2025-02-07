mod control;
mod display;
mod instructions;
mod logs;
mod registers;
mod serial;

use crate::ExclusiveState;
use eframe::egui::{Id, InnerResponse, Ui, Window};
use serde::{Deserialize, Serialize};

trait WindowUi {
    fn build<'open>(&mut self, open: &'open mut bool) -> Window<'open>;
    fn show(&mut self, state: &mut ExclusiveState, ui: &mut Ui);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppWindowKind {
    Control,
    Display,
    Instructions,
    Logs,
    Registers,
    Vram,
    Serial,
}

pub struct AppWindow {
    id: Id,
    kind: AppWindowKind,
    open: bool,
    window: Box<dyn WindowUi>,
}

impl AppWindow {
    pub fn open(kind: AppWindowKind, id: Id) -> Self {
        Self {
            id,
            kind,
            window: match kind {
                AppWindowKind::Control => Box::new(control::Control::new(id)),
                AppWindowKind::Display => Box::new(display::Display::new(id, false)),
                AppWindowKind::Instructions => Box::new(instructions::InstructionViewer::new(id)),
                AppWindowKind::Logs => Box::new(logs::LogViewer::new(id)),
                AppWindowKind::Registers => Box::new(registers::Registers::new(id)),
                AppWindowKind::Vram => Box::new(display::Display::new(id, true)),
                AppWindowKind::Serial => Box::new(serial::Serial::new(id)),
            },
            open: true,
        }
    }

    pub fn show(
        &mut self,
        state: &mut ExclusiveState,
        ui: &mut Ui,
    ) -> Option<InnerResponse<Option<()>>> {
        let container = self
            .window
            .build(&mut self.open)
            .id(self.id)
            .constrain_to(ui.max_rect());

        container.show(ui.ctx(), |ui| {
            self.window.show(state, ui);
        })
    }

    pub fn id(&self) -> Id {
        self.id
    }

    pub fn kind(&self) -> AppWindowKind {
        self.kind
    }
}
