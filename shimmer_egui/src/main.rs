mod colors;
mod tab;
mod util;

use eframe::{
    egui::{self, menu},
    epaint::Rounding,
};
use egui_dock::{DockArea, DockState};
use parking_lot::{Mutex, MutexGuard};
use shimmer_core::{cpu::Reg, PSX};
use std::{sync::Arc, time::Duration};
use tab::Tab;
use tab::{
    breakpoints::Breakpoints, instruction_viewer::InstructionViewer, log_viewer::LogViewer,
    memory_viewer::MemoryViewer, screen::Screen, system_control::SystemControl, tty::Terminal,
};
use tinylog::{drain::buf::RecordBuf, logger::LoggerFamily};
use util::Timer;

struct Timing {
    running_timer: Timer,
    emulated_time: Duration,
}

/// Data that's shared between the GUI and the emulation thread.
struct Shared {
    psx: PSX,
    timing: Timing,

    running: bool,
    breakpoints: Vec<u32>,
    should_reset: bool,

    log_family: LoggerFamily,
    log_records: RecordBuf,

    // ui -- should this be here?
    terminal_output: String,
    alternative_names: bool,
}

impl Shared {
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
        Shared {
            psx: PSX::with_bios(bios, root_logger),
            timing: Timing {
                running_timer: Timer::new(),
                emulated_time: Duration::ZERO,
            },

            running: false,
            breakpoints: Vec::new(),
            should_reset: false,

            log_family,
            log_records,

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
                self.shared.timing.running_timer.resume();
            } else {
                self.shared.timing.running_timer.pause();
            }

            self.current_running = self.shared.running;
        }
    }

    pub fn catch_up(&mut self) {
        let time_behind = self
            .shared
            .timing
            .running_timer
            .elapsed()
            .saturating_sub(self.shared.timing.emulated_time);

        let cycles_to_run = 33_870_000 as f64 * time_behind.as_secs_f64() + self.fractional_cycles;

        self.fractional_cycles = cycles_to_run.fract();
        let full_cycles_to_run = cycles_to_run as u64;

        let mut cycles_left = full_cycles_to_run;
        while cycles_left > 0 {
            let taken = 2048.min(cycles_left);
            cycles_left -= taken;

            for _ in 0..taken {
                self.shared.psx.cycle();
                if self.shared.psx.cpu.to_exec().1 == 0xB0 {
                    let call = self.shared.psx.cpu.regs().read(Reg::R9);
                    if call == 0x3D {
                        let char = self.shared.psx.cpu.regs().read(Reg::A0);
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
        self.shared.timing.emulated_time += time_behind
            .mul_f64((emulated_cycles as f64 / full_cycles_to_run as f64).max(f64::EPSILON));
    }

    pub fn cycle(&mut self) {
        self.prologue();

        // TODO: this is stupid, do something better
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

struct App {
    dock: DockState<tab::Instance>,
    shared: Arc<Mutex<Shared>>,
    id: u64,
}

impl App {
    fn new(_ctx: &eframe::CreationContext<'_>) -> Self {
        let shared = setup_emulation_thread();
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
            shared,
            id: id + 1,
        }
    }

    fn open_tab<T>(&mut self)
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

        let surface = self.dock.main_surface_mut();
        surface.split_right(
            egui_dock::NodeIndex::root(),
            0.5,
            vec![tab::Instance {
                id: self.id,
                tab: Box::new(T::new(self.id)),
            }],
        );

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
            &mut tab::Viewer {
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
        "shimmer - psx",
        native_options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    );

    if let Err(e) = result {
        eprintln!("{e:?}");
    }
}
