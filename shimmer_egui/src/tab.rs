pub mod breakpoints;
pub mod instruction_viewer;
pub mod log_viewer;
pub mod memory_viewer;
pub mod screen;
pub mod system_control;
pub mod tty;

use crate::Shared;
use eframe::egui;
use std::any::Any;

pub struct Context<'psx> {
    pub shared: &'psx mut Shared,
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

pub struct Viewer<'shared> {
    pub shared: &'shared mut Shared,
    pub focused_tab_id: Option<u64>,
}

impl<'psx> egui_dock::TabViewer for Viewer<'psx> {
    type Tab = Instance;

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        egui::Id::new(tab.id)
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.tab.title()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        let ctx = Context {
            shared: &mut self.shared,
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
}
