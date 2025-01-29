mod cli;
mod colors;
mod emulation;
mod tab;
mod util;

use clap::Parser;
use cli::Cli;
use crossbeam::sync::{Parker, Unparker};
use eframe::{
    egui::{self, menu},
    egui_wgpu::{RenderState, WgpuSetup},
    epaint::Rounding,
    wgpu,
};
use egui_dock::{DockArea, DockState, NodeIndex, SurfaceIndex};
use parking_lot::Mutex;
use shimmer_core::Emulator;
use shimmer_wgpu::WgpuRenderer;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tab::{
    Tab, breakpoints::Breakpoints, instruction_viewer::InstructionViewer, log_viewer::LogViewer,
    memory_viewer::MemoryViewer, screen::Screen, system_control::SystemControl, tty::Terminal,
};
use tinylog::{drain::buf::RecordBuf, logger::LoggerFamily};
use util::Timer;

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

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
    emulator: Emulator,
    renderer: WgpuRenderer,
    timing: Timing,
    controls: Controls,

    log_family: LoggerFamily,
    log_records: RecordBuf,
}

impl ExclusiveState {
    fn new(render_state: &RenderState, bios: Vec<u8>, sideload_rom: Option<Vec<u8>>) -> Self {
        let log_records = RecordBuf::new();
        let log_family = LoggerFamily::builder()
            .with_drain(log_records.drain())
            .build();

        let level = if cfg!(debug_assertions) {
            tinylog::Level::Debug
        } else {
            tinylog::Level::Info
        };
        let root_logger = log_family.logger("psx", level);

        let renderer_config = shimmer_wgpu::Config {
            display_tex_format: render_state.target_format,
        };
        let device = Arc::clone(&render_state.device);
        let queue = Arc::clone(&render_state.queue);
        let renderer = WgpuRenderer::new(
            device,
            queue,
            log_family.logger("wgpu-renderer", tinylog::Level::Trace),
            renderer_config,
        );

        let mut emulator = Emulator::new(bios, root_logger, renderer.clone());
        if let Some(rom) = sideload_rom {
            use shimmer_core::binrw::BinReaderExt;
            let exe: shimmer_core::exe::Executable = std::io::Cursor::new(rom).read_le().unwrap();
            emulator.psx_mut().memory.sideload = Some(exe);
        }

        Self {
            emulator,
            renderer,
            timing: Timing {
                running_timer: Timer::new(),
                emulated_time: Duration::ZERO,
            },
            controls: Controls {
                running: false,
                breakpoints: Vec::new(),
                alternative_names: true,
            },

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
    bios: Vec<u8>,
    exclusive: Mutex<ExclusiveState>,
    shared: SharedState,
}

impl State {
    fn new(render_state: &RenderState, bios: Vec<u8>, sideload_rom: Option<Vec<u8>>) -> Self {
        Self {
            bios: bios.clone(),
            exclusive: Mutex::new(ExclusiveState::new(render_state, bios, sideload_rom)),
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
    fn new(cc: &eframe::CreationContext<'_>, cli: Cli) -> Self {
        let bios_path = cli.args.bios.clone().unwrap_or("resources/BIOS.BIN".into());
        let bios = std::fs::read(bios_path).expect("bios file exists");

        let rom_path = cli.args.input.clone();
        let rom = rom_path.map(|path| std::fs::read(path).expect("rom file exists"));

        let state = Arc::new(State::new(
            cc.wgpu_render_state.as_ref().unwrap(),
            bios,
            rom,
        ));

        let parker = Parker::new();
        let unparker = parker.unparker().clone();

        std::thread::Builder::new()
            .name("emulator thread".to_owned())
            .spawn({
                let state = state.clone();
                || emulation::run(state, parker)
            })
            .unwrap();

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
        surface[mem_viewer].append_tab(tab!(LogViewer));
        let [_, _] = surface.split_below(mem_viewer, 0.63, tab!(vec Terminal));
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
        let mut dump = false;
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

                if ui.button("Dump Ram").clicked() {
                    dump = true;
                }

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

        if dump {
            std::fs::write("dump.bin", exclusive.emulator.psx().memory.ram.as_slice()).unwrap();
        }

        if reset {
            let old = std::mem::replace(
                &mut *exclusive,
                ExclusiveState::new(
                    _frame.wgpu_render_state().unwrap(),
                    self.state.bios.clone(),
                    None,
                ),
            );
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
    let cli = Cli::parse();

    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport.min_inner_size = Some(egui::Vec2::new(500.0, 500.0));
    native_options.viewport.inner_size = Some(egui::Vec2::new(1333.0, 1000.0));
    native_options.viewport.maximized = Some(true);
    native_options.wgpu_options.wgpu_setup = WgpuSetup::CreateNew {
        supported_backends: wgpu::Backends::default(),
        power_preference: wgpu::PowerPreference::HighPerformance,
        device_descriptor: Arc::new(|_| {
            // for renderdoc
            wgpu::DeviceDescriptor {
                label: Some("device"),
                required_features: wgpu::Features::default(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            }
        }),
    };

    let result = eframe::run_native(
        "shimmer - psx",
        native_options,
        Box::new(|cc| Ok(Box::new(App::new(cc, cli)))),
    );

    if let Err(e) = result {
        eprintln!("{e:?}");
    }
}
