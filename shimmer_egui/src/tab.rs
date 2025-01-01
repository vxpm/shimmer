pub mod breakpoints;
pub mod instruction_viewer;
pub mod log_viewer;
pub mod memory_viewer;
pub mod screen;
pub mod system_control;
pub mod tty;

use crate::ExclusiveState;
use eframe::egui::{self, UiBuilder, Vec2};
use egui_dock::{NodeIndex, SurfaceIndex};
use std::any::Any;

pub struct Context<'psx> {
    pub exclusive: &'psx mut ExclusiveState,
    pub is_focused: bool,
}

/// Trait for tabs in the GUI.
pub trait Tab {
    fn new(id: u64) -> Self
    where
        Self: Sized;

    fn title(&mut self) -> egui::WidgetText;

    fn ui(&mut self, ui: &mut egui::Ui, ctx: Context);

    fn closable(&mut self) -> bool {
        true
    }

    fn multiple_allowed() -> bool
    where
        Self: Sized,
    {
        false
    }
}

pub trait AnyShimmerTab: Any + Tab {
    fn as_any(&self) -> &dyn Any;
}

impl<T> AnyShimmerTab for T
where
    T: Any + Tab,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct Instance {
    pub id: u64,
    pub tab: Box<dyn AnyShimmerTab>,
}

impl PartialEq for Instance {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Clone, Copy)]
pub enum TabToAdd {
    Logs,
    Terminal,
    MemoryViewer,
    InstructionViewer,
}

pub struct Viewer<'state> {
    pub exclusive: &'state mut ExclusiveState,
    pub focused_tab_id: Option<u64>,
    pub to_add: Option<(SurfaceIndex, NodeIndex, TabToAdd)>,
}

impl egui_dock::TabViewer for Viewer<'_> {
    type Tab = Instance;

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        egui::Id::new(tab.id)
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.tab.title()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        let ctx = Context {
            exclusive: self.exclusive,
            is_focused: self
                .focused_tab_id
                .map(|focused_tab_id| focused_tab_id == tab.id)
                .unwrap_or(false),
        };

        tab.tab.ui(ui, ctx);
    }

    fn closeable(&mut self, tab: &mut Self::Tab) -> bool {
        tab.tab.closable()
    }

    fn allowed_in_windows(&self, _tab: &mut Self::Tab) -> bool {
        false
    }

    fn scroll_bars(&self, _tab: &Self::Tab) -> [bool; 2] {
        [false, false]
    }

    fn add_popup(
        &mut self,
        ui: &mut egui::Ui,
        surface: egui_dock::SurfaceIndex,
        node: egui_dock::NodeIndex,
    ) {
        let (_, rect) = ui.allocate_space(Vec2::new(100.0, 100.0));

        // ugly currently but who cares
        ui.allocate_new_ui(UiBuilder::new().max_rect(rect), |ui| {
            if ui.button("Logs").clicked() {
                self.to_add = Some((surface, node, TabToAdd::Logs));
            }

            if ui.button("Terminal").clicked() {
                self.to_add = Some((surface, node, TabToAdd::Terminal));
            }

            if ui.button("Memory Viewer").clicked() {
                self.to_add = Some((surface, node, TabToAdd::MemoryViewer));
            }

            if ui.button("Instruction Viewer").clicked() {
                self.to_add = Some((surface, node, TabToAdd::InstructionViewer));
            }
        });
    }
}
