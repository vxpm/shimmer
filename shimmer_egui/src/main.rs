mod breakpoints;
mod colors;
mod instruction_viewer;
mod log_viewer;
mod memory_viewer;
mod screen;
mod system_control;
mod tty;
mod util;

use eframe::{
    egui::{self, menu},
    epaint::Rounding,
};
use egui_dock::{DockArea, DockState};
use instruction_viewer::InstructionViewer;
use log_viewer::LogViewer;
use memory_viewer::MemoryViewer;
use parking_lot::{Mutex, MutexGuard};
use screen::Screen;
use shimmer_core::{cpu::Reg, PSX};
use std::{any::Any, sync::Arc, time::Duration};
use system_control::SystemControl;
use tty::Terminal;
use util::Timer;

use crate::breakpoints::Breakpoints;

/// Data that's shared between the GUI and the emulation thread.
struct Shared {
    psx: PSX,
    running: bool,
    running_timer: Timer,
    emulated_time: Duration,
    breakpoints: Vec<u32>,
    should_reset: bool,

    // ui
    terminal_output: String,
    alternative_names: bool,
}

impl Shared {
    fn new() -> Self {
        let bios = std::fs::read("BIOS.BIN").expect("bios in directory");
        Shared {
            psx: PSX::with_bios(bios),
            running: false,
            running_timer: Timer::new(),
            emulated_time: Duration::ZERO,
            breakpoints: Vec::new(),
            should_reset: false,
            terminal_output: String::new(),
            alternative_names: true,
        }
    }
}

struct EmulationCtx<'shared> {
    shared: parking_lot::MutexGuard<'shared, Shared>,
    current_running: bool,
    fractional_cycles: f64,
}

impl<'shared> EmulationCtx<'shared> {
    pub fn prologue(&mut self) {
        if self.shared.should_reset {
            let old_shared = std::mem::replace(&mut *self.shared, Shared::new());
            self.shared.breakpoints = old_shared.breakpoints;
            self.current_running = false;
        }

        if self.shared.running != self.current_running {
            if self.shared.running {
                self.shared.running_timer.resume();
            } else {
                self.shared.running_timer.pause();
            }

            self.current_running = self.shared.running;
        }
    }

    pub fn catch_up(&mut self) {
        let time_behind = self
            .shared
            .running_timer
            .elapsed()
            .saturating_sub(self.shared.emulated_time);

        let cycles_to_run = 33_870_000 as f64 * time_behind.as_secs_f64() + self.fractional_cycles;

        self.fractional_cycles = cycles_to_run.fract();
        let full_cycles_to_run = cycles_to_run as u64;

        let mut cycles_left = full_cycles_to_run;
        while cycles_left > 0 {
            let taken = 2048.min(cycles_left);
            cycles_left -= taken;

            for _ in 0..taken {
                let _cycle_info = self.shared.psx.cycle();

                if self.shared.psx.cpu.to_exec().1 == 0xB0 {
                    let call = self.shared.psx.cpu.regs().read(Reg::R9);
                    if call == 0x3D {
                        let char = self.shared.psx.cpu.regs().read(Reg::R4);
                        if let Ok(char) = char::try_from(char) {
                            self.shared.terminal_output.push(char);
                        }
                    }
                }

                if self
                    .shared
                    .breakpoints
                    .contains(&self.shared.psx.cpu.to_exec().1.value())
                {
                    self.shared.running = false;
                    break;
                }
            }

            MutexGuard::bump(&mut self.shared);

            if !self.shared.running {
                break;
            }
        }

        let emulated_cycles = full_cycles_to_run - cycles_left;
        self.shared.emulated_time += time_behind
            .mul_f64((emulated_cycles as f64 / full_cycles_to_run as f64).max(f64::EPSILON));
    }

    pub fn cycle(&mut self) {
        self.prologue();

        if self.shared.running {
            self.catch_up();
        }

        MutexGuard::bump(&mut self.shared);
    }
}

fn setup_emulation_thread() -> Arc<Mutex<Shared>> {
    let shared = Arc::new(Mutex::new(Shared::new()));

    std::thread::spawn({
        let shared = shared.clone();
        move || {
            let shared = shared.lock();
            let mut ctx = EmulationCtx {
                current_running: shared.running,
                fractional_cycles: 0.0,
                shared,
            };

            loop {
                ctx.cycle();
            }
        }
    });

    shared
}

struct TabContext<'psx> {
    shared: &'psx mut Shared,
    is_focused: bool,
}

/// Trait for tabs in the GUI.
trait VioletTab {
    fn new(id: u64) -> Self
    where
        Self: Sized;

    fn title(&mut self) -> egui::WidgetText;

    fn ui(&mut self, ui: &mut egui::Ui, ctx: TabContext);

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

trait AnyVioletTab: Any + VioletTab {
    fn as_any(&self) -> &dyn Any;
    fn as_tab(&self) -> &dyn VioletTab;
}

impl<T> AnyVioletTab for T
where
    T: Any + VioletTab,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_tab(&self) -> &dyn VioletTab {
        self
    }
}

struct TabWithId {
    id: u64,
    tab: Box<dyn AnyVioletTab>,
}

impl PartialEq for TabWithId {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

struct VioletTabViewer<'shared> {
    shared: &'shared mut Shared,
    focused_tab_id: Option<u64>,
}

impl<'psx> egui_dock::TabViewer for VioletTabViewer<'psx> {
    type Tab = TabWithId;

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        egui::Id::new(tab.id)
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.tab.title()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        let ctx = TabContext {
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

/// Contains the GUI state.
struct VioletEgui {
    dock: DockState<TabWithId>,
    shared: Arc<Mutex<Shared>>,
    id: u64,
}

impl VioletEgui {
    fn new(_ctx: &eframe::CreationContext<'_>) -> Self {
        let shared = setup_emulation_thread();
        let mut dock: DockState<TabWithId> = DockState::new(vec![]);

        let mut id = 0;
        macro_rules! tab {
            ($t:ty) => {{
                id += 1;
                TabWithId {
                    id,
                    tab: Box::new(<$t>::new(id)),
                }
            }};
            (vec $t:ty) => {{
                id += 1;
                vec![TabWithId {
                    id,
                    tab: Box::new(<$t>::new(id)),
                }]
            }};
        }

        // setup default layout
        let surface = dock.main_surface_mut();

        surface.push_to_first_leaf(tab!(SystemControl));
        let [system_control, mem_viewer] =
            surface.split_left(egui_dock::NodeIndex::root(), 0.77, tab!(vec Screen));
        surface[mem_viewer].append_tab(tab!(MemoryViewer));
        let [_, log_viewer] = surface.split_below(mem_viewer, 0.63, tab!(vec LogViewer));
        surface[log_viewer].append_tab(tab!(Terminal));
        surface.split_below(system_control, 0.47, tab!(vec Breakpoints));
        surface.split_right(
            egui_dock::NodeIndex::root(),
            0.81,
            tab!(vec InstructionViewer),
        );

        Self {
            dock,
            shared,
            id: id + 1,
        }
    }

    fn open_tab<T>(&mut self)
    where
        T: AnyVioletTab,
    {
        if !T::multiple_allowed() {
            let already_open = self
                .dock
                .iter_all_tabs()
                .map(|(_, t)| t)
                .any(|t| t.tab.as_any().type_id() == std::any::TypeId::of::<T>());

            if already_open {
                return;
            }
        }

        let surface = self.dock.main_surface_mut();
        surface.split_right(
            egui_dock::NodeIndex::root(),
            0.5,
            vec![TabWithId {
                id: self.id,
                tab: Box::new(T::new(self.id)),
            }],
        );

        self.id += 1;
    }
}

impl eframe::App for VioletEgui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut reset = false;
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open ROM").clicked() {
                        // TODO
                    }
                });

                ui.separator();

                ui.menu_button("Tabs", |ui| {
                    if ui.button("System Control").clicked() {
                        self.open_tab::<SystemControl>();
                        ui.close_menu();
                    }

                    if ui.button("Breakpoints").clicked() {
                        self.open_tab::<Breakpoints>();
                        ui.close_menu();
                    }

                    if ui.button("Screen").clicked() {
                        self.open_tab::<Screen>();
                        ui.close_menu();
                    }

                    if ui.button("Instruction Viewer").clicked() {
                        self.open_tab::<InstructionViewer>();
                        ui.close_menu();
                    }

                    if ui.button("Memory Viewer").clicked() {
                        self.open_tab::<MemoryViewer>();
                        ui.close_menu();
                    }

                    if ui.button("Logs").clicked() {
                        self.open_tab::<LogViewer>();
                        ui.close_menu();
                    }

                    if ui.button("Terminal").clicked() {
                        self.open_tab::<Terminal>();
                        ui.close_menu();
                    }

                    ui.menu_button("Presets", |ui| {
                        ui.label("none yet");
                    });
                });

                ui.separator();

                if ui.button("Hard Reset").clicked() {
                    reset = true;
                }
            });
        });

        let focused_tab_id = self.dock.find_active_focused().map(|(_, t)| t.id);
        let mut style = egui_dock::Style::from_egui(&ctx.style());
        style.tab_bar.rounding = Rounding::ZERO;
        style.tab_bar.bg_fill = ctx.style().visuals.panel_fill;
        style.dock_area_padding = None;

        let mut shared = self.shared.lock();
        if reset {
            shared.should_reset = true;
        }

        DockArea::new(&mut self.dock).style(style).show(
            ctx,
            &mut VioletTabViewer {
                shared: &mut shared,
                focused_tab_id,
            },
        );

        if shared.running {
            ctx.request_repaint_after(Duration::from_secs_f64(1.0 / 75.0));
        }

        std::mem::drop(shared);
    }
}

fn main() {
    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport.min_inner_size = Some(egui::Vec2::new(500.0, 500.0));
    native_options.viewport.inner_size = Some(egui::Vec2::new(1333.0, 1000.0));
    native_options.viewport.maximized = Some(true);

    let result = eframe::run_native(
        "violet - psx",
        native_options,
        Box::new(|cc| Box::new(VioletEgui::new(cc))),
    );

    if let Err(e) = result {
        eprintln!("{e:?}");
    }
}
