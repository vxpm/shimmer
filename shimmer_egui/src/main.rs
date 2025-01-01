mod colors;
mod emulation;
mod tab;
mod util;

use crossbeam::sync::{Parker, Unparker};
use eframe::{
    egui::{self, menu},
    epaint::Rounding,
};
use egui_dock::{DockArea, DockState, NodeIndex, SurfaceIndex};
use parking_lot::Mutex;
use shimmer_core::PSX;
use std::{
    io::Cursor,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tab::Tab;
use tab::{
    breakpoints::Breakpoints, instruction_viewer::InstructionViewer, log_viewer::LogViewer,
    memory_viewer::MemoryViewer, screen::Screen, system_control::SystemControl, tty::Terminal,
};
use tinylog::{drain::buf::RecordBuf, logger::LoggerFamily};
use util::Timer;

/// Variables related to timing.
struct Timing {
    running_timer: Timer,
    emulated_time: Duration,
}

/// Variables related to controlling the emulation or the GUI.
struct Controls {
    running: bool,
    breakpoints: Vec<u32>,
    alternative_names: bool,
}

/// State shared between the GUI and emulation threads that is locked behind a mutex.
struct ExclusiveState {
    psx: PSX,
    timing: Timing,
    controls: Controls,
    terminal_output: String,

    log_family: LoggerFamily,
    log_records: RecordBuf,
}

impl ExclusiveState {
    fn new() -> Self {
        let bios = std::fs::read("BIOS.BIN").expect("bios in directory");

        let log_records = RecordBuf::new();
        let log_family = LoggerFamily::builder()
            .with_drain(log_records.drain())
            .build();

        let level = if cfg!(debug_assertions) {
            tinylog::Level::Trace
        } else {
            tinylog::Level::Info
        };
        let root_logger = log_family.logger("psx", level);

        let mut psx = PSX::with_bios(bios, root_logger);

        // use shimmer_core::binrw::BinReaderExt;
        // let exe = std::fs::read("psxtest_cpu.exe").unwrap();
        // let exe: shimmer_core::exe::Executable = Cursor::new(exe).read_le().unwrap();
        // psx.memory.sideload = Some(exe);

        Self {
            psx,
            timing: Timing {
                running_timer: Timer::new(),
                emulated_time: Duration::ZERO,
            },
            controls: Controls {
                running: false,
                breakpoints: Vec::new(),
                alternative_names: true,
            },
            terminal_output: String::new(),

            log_family,
            log_records,
        }
    }
}

/// State shared between the GUI and emulation threads that is not locked behind a mutex.
#[derive(Default)]
struct SharedState {
    should_advance: AtomicBool,
}

/// State shared between the GUI and emulation threads.
struct State {
    exclusive: Mutex<ExclusiveState>,
    shared: SharedState,
}

impl Default for State {
    fn default() -> Self {
        Self {
            exclusive: Mutex::new(ExclusiveState::new()),
            shared: Default::default(),
        }
    }
}

struct App {
    dock: DockState<tab::Instance>,
    state: Arc<State>,
    id: u64,
    unparker: Unparker,
}

impl App {
    fn new(_ctx: &eframe::CreationContext<'_>) -> Self {
        let state = Arc::new(State::default());
        let parker = Parker::new();
        let unparker = parker.unparker().clone();
        std::thread::spawn({
            let state = state.clone();
            || emulation::run(state, parker)
        });

        let mut dock: DockState<tab::Instance> = DockState::new(vec![]);
        let mut id = 0;
        macro_rules! tab {
            ($t:ty) => {{
                id += 1;
                tab::Instance {
                    id,
                    tab: Box::new(<$t>::new(id)),
                }
            }};
            (vec $t:ty) => {{
                id += 1;
                vec![tab::Instance {
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
            state,
            id: id + 1,
            unparker,
        }
    }

    fn open_tab<T>(&mut self, node: Option<(SurfaceIndex, NodeIndex)>)
    where
        T: tab::AnyShimmerTab,
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

        if let Some((surface, node)) = node {
            self.dock.set_focused_node_and_surface((surface, node));
        }

        self.dock.push_to_focused_leaf(tab::Instance {
            id: self.id,
            tab: Box::new(T::new(self.id)),
        });

        self.id += 1;
    }
}

impl eframe::App for App {
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
                        self.open_tab::<SystemControl>(None);
                        ui.close_menu();
                    }

                    if ui.button("Breakpoints").clicked() {
                        self.open_tab::<Breakpoints>(None);
                        ui.close_menu();
                    }

                    if ui.button("Screen").clicked() {
                        self.open_tab::<Screen>(None);
                        ui.close_menu();
                    }

                    if ui.button("Instruction Viewer").clicked() {
                        self.open_tab::<InstructionViewer>(None);
                        ui.close_menu();
                    }

                    if ui.button("Memory Viewer").clicked() {
                        self.open_tab::<MemoryViewer>(None);
                        ui.close_menu();
                    }

                    if ui.button("Logs").clicked() {
                        self.open_tab::<LogViewer>(None);
                        ui.close_menu();
                    }

                    if ui.button("Terminal").clicked() {
                        self.open_tab::<Terminal>(None);
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

        self.state
            .shared
            .should_advance
            .store(false, Ordering::Relaxed);
        let mut exclusive = self.state.exclusive.lock();

        if reset {
            let old = std::mem::replace(&mut *exclusive, ExclusiveState::new());
            exclusive.controls.breakpoints = old.controls.breakpoints;
        }

        let to_add = {
            let mut viewer = tab::Viewer {
                exclusive: &mut exclusive,
                focused_tab_id,
                to_add: None,
            };

            DockArea::new(&mut self.dock)
                .style(style)
                .show_add_buttons(true)
                .show_add_popup(true)
                .show(ctx, &mut viewer);

            viewer.to_add
        };

        if exclusive.controls.running {
            exclusive.timing.running_timer.resume();
            ctx.request_repaint_after(Duration::from_secs_f64(1.0 / 60.0));

            self.state
                .shared
                .should_advance
                .store(true, Ordering::Relaxed);
            self.unparker.unpark();
        } else {
            exclusive.timing.running_timer.pause();
        }

        std::mem::drop(exclusive);
        if let Some((surface, node, tab)) = to_add {
            let node = Some((surface, node));
            match tab {
                tab::TabToAdd::Logs => self.open_tab::<LogViewer>(node),
                tab::TabToAdd::Terminal => self.open_tab::<Terminal>(node),
                tab::TabToAdd::MemoryViewer => self.open_tab::<MemoryViewer>(node),
                tab::TabToAdd::InstructionViewer => self.open_tab::<InstructionViewer>(node),
            }
        }
    }
}

fn main() {
    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport.min_inner_size = Some(egui::Vec2::new(500.0, 500.0));
    native_options.viewport.inner_size = Some(egui::Vec2::new(1333.0, 1000.0));
    native_options.viewport.maximized = Some(false);

    let result = eframe::run_native(
        "shimmer - psx",
        native_options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    );

    if let Err(e) = result {
        eprintln!("{e:?}");
    }
}
