#![feature(random)]

mod cli;
mod emulation;
mod util;
mod windows;

use clap::Parser;
use cli::Cli;
use crossbeam::sync::{Parker, Unparker};
use eframe::{
    egui::{self, Frame, Id, Style, menu},
    egui_wgpu::{RenderState, WgpuSetup, WgpuSetupCreateNew},
    wgpu,
};
use egui_file_dialog::FileDialog;
use gilrs::{Button, Gilrs};
use parking_lot::Mutex;
use shimmer_core::Emulator;
use shimmer_wgpu::WgpuRenderer;
use std::{
    path::PathBuf,
    random::random,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tinylog::{drain::buf::RecordBuf, logger::LoggerFamily};
use util::Timer;
use windows::{AppWindow, AppWindowKind};

/// Variables related to timing.
struct Timing {
    running_timer: Timer,
    emulated_time: Duration,
}

/// Variables related to controlling the emulation or the GUI.
struct Controls {
    running: bool,
    #[expect(dead_code, reason = "temporary")]
    breakpoints: Vec<u32>,
    alternative_names: bool,
    gamepad_input: Gilrs,
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
    fn new(render_state: &RenderState, config: Config) -> Self {
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
        let device = render_state.device.clone();
        let queue = render_state.queue.clone();
        let renderer = WgpuRenderer::new(
            device,
            queue,
            log_family.logger("wgpu-renderer", tinylog::Level::Trace),
            renderer_config,
        );

        let bios = std::fs::read(config.bios_path).expect("should be a valid bios path");
        let emulator_config = shimmer_core::Config {
            bios,
            rom_path: config.rom_path,
            logger: root_logger,
        };

        let mut emulator = Emulator::new(emulator_config, renderer.clone()).unwrap();
        if let Some(path) = config.sideload_exe_path {
            use shimmer_core::binrw::BinReaderExt;
            let exe = std::fs::read(path).expect("should be a valid sideload exe path");
            let exe: shimmer_core::exe::Executable = std::io::Cursor::new(exe).read_le().unwrap();
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
                gamepad_input: Gilrs::new().unwrap(),
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
    exclusive: Mutex<ExclusiveState>,
    shared: SharedState,
}

impl State {
    fn new(render_state: &RenderState, config: Config) -> Self {
        Self {
            exclusive: Mutex::new(ExclusiveState::new(render_state, config)),
            shared: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
struct Config {
    bios_path: PathBuf,
    rom_path: Option<PathBuf>,
    sideload_exe_path: Option<PathBuf>,
}

struct App {
    _config: Config,
    state: Arc<State>,
    unparker: Unparker,

    windows: Vec<AppWindow>,
    file_dialog: FileDialog,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, cli: Cli) -> Self {
        let bios_path = cli.args.bios.clone().unwrap_or("resources/BIOS.BIN".into());
        let rom_path = cli.args.input.clone();
        let sideload_exe_path = cli.args.sideload_exe.clone();
        let config = Config {
            bios_path,
            rom_path,
            sideload_exe_path,
        };

        let state = Arc::new(State::new(
            cc.wgpu_render_state.as_ref().unwrap(),
            config.clone(),
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

        let windows: Vec<(AppWindowKind, Id)> = cc
            .storage
            .as_ref()
            .and_then(|s| s.get_string("windows"))
            .and_then(|s| ron::from_str(&s).ok())
            .unwrap_or_default();

        let windows = windows
            .into_iter()
            .map(|(kind, id)| AppWindow::open(kind, id))
            .collect();

        Self {
            _config: config,
            state,
            unparker,

            windows,
            file_dialog: FileDialog::new().as_modal(true),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open game directory").clicked() {
                        self.file_dialog.pick_directory();
                    }
                });

                ui.separator();

                if ui.button("Organize").clicked() {
                    ui.ctx().memory_mut(|mem| mem.reset_areas());
                }

                ui.separator();

                ui.label(format!(
                    "Rendering time: {:?}",
                    Duration::from_secs_f32(frame.info().cpu_usage.unwrap_or_default())
                ));
            });
        });

        self.state
            .shared
            .should_advance
            .store(false, Ordering::Relaxed);
        let mut state = self.state.exclusive.lock();

        egui::CentralPanel::default()
            .frame(Frame::canvas(&Style::default()))
            .show(ctx, |ui| {
                self.file_dialog.update(ctx);
                self.windows.retain_mut(|window| {
                    let response = window.show(&mut state, ui);
                    response.is_some()
                });
            })
            .response
            .context_menu(|ui| {
                ui.menu_button("🖵 Windows", |ui| {
                    if ui.button("Control").clicked() {
                        self.windows.push(AppWindow::open(
                            AppWindowKind::Control,
                            Id::new(random::<u64>()),
                        ));
                        ui.close_menu();
                    }

                    if ui.button("Display").clicked() {
                        self.windows.push(AppWindow::open(
                            AppWindowKind::Display,
                            Id::new(random::<u64>()),
                        ));
                        ui.close_menu();
                    }

                    if ui.button("Instructions").clicked() {
                        self.windows.push(AppWindow::open(
                            AppWindowKind::Instructions,
                            Id::new(random::<u64>()),
                        ));
                        ui.close_menu();
                    }

                    if ui.button("Logs").clicked() {
                        self.windows.push(AppWindow::open(
                            AppWindowKind::Logs,
                            Id::new(random::<u64>()),
                        ));
                        ui.close_menu();
                    }

                    if ui.button("Registers").clicked() {
                        self.windows.push(AppWindow::open(
                            AppWindowKind::Registers,
                            Id::new(random::<u64>()),
                        ));
                        ui.close_menu();
                    }

                    if ui.button("VRAM").clicked() {
                        self.windows.push(AppWindow::open(
                            AppWindowKind::Vram,
                            Id::new(random::<u64>()),
                        ));
                        ui.close_menu();
                    }
                });
            });

        if state.controls.running {
            state.timing.running_timer.resume();
            ctx.request_repaint_after(Duration::from_secs_f64(1.0 / 60.0));

            self.state
                .shared
                .should_advance
                .store(true, Ordering::Relaxed);
            self.unparker.unpark();
        } else {
            state.timing.running_timer.pause();
        }

        while let Some(event) = state.controls.gamepad_input.next_event() {
            let input = &mut state.emulator.psx_mut().sio0.input;
            match event.event {
                gilrs::EventType::ButtonChanged(button, value, _) => {
                    let level = value > 0.0;
                    match button {
                        Button::South => {
                            input.set_cross(level);
                        }
                        Button::East => {
                            input.set_circle(level);
                        }
                        Button::North => {
                            input.set_triangle(level);
                        }
                        Button::West => {
                            input.set_square(level);
                        }
                        Button::LeftTrigger => {
                            input.set_l1(level);
                        }
                        Button::LeftTrigger2 => {
                            input.set_l2(level);
                        }
                        Button::RightTrigger => {
                            input.set_r1(level);
                        }
                        Button::RightTrigger2 => {
                            input.set_r2(level);
                        }
                        Button::Select => {
                            input.set_select(level);
                            input.set_start(!input.start());
                        }
                        Button::Start => {
                            input.set_start(level);
                        }
                        Button::LeftThumb => {
                            input.set_l3(level);
                        }
                        Button::RightThumb => {
                            input.set_r3(level);
                        }
                        Button::DPadUp => {
                            input.set_joy_up(level);
                        }
                        Button::DPadDown => {
                            input.set_joy_down(level);
                        }
                        Button::DPadLeft => {
                            input.set_joy_left(level);
                        }
                        Button::DPadRight => {
                            input.set_joy_right(level);
                        }
                        _ => (),
                    }
                }
                // gilrs::EventType::AxisChanged(axis, _, code) => todo!(),
                // gilrs::EventType::Connected => todo!(),
                // gilrs::EventType::Disconnected => todo!(),
                // gilrs::EventType::Dropped => todo!(),
                // gilrs::EventType::ForceFeedbackEffectCompleted => todo!(),
                _ => (),
            }
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let windows = self
            .windows
            .iter()
            .map(|w| (w.kind(), w.id()))
            .collect::<Vec<_>>();

        storage.set_string("windows", ron::to_string(&windows).unwrap());
    }
}

fn main() {
    let cli = Cli::parse();

    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport.min_inner_size = Some(egui::Vec2::new(500.0, 500.0));
    native_options.viewport.maximized = Some(true);
    native_options.wgpu_options.wgpu_setup = WgpuSetup::CreateNew(WgpuSetupCreateNew {
        instance_descriptor: wgpu::InstanceDescriptor::from_env_or_default(),
        power_preference: wgpu::PowerPreference::HighPerformance,
        native_adapter_selector: None,
        device_descriptor: Arc::new(|_| wgpu::DeviceDescriptor {
            label: Some("device"),
            required_features: wgpu::Features::default(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
        }),
        trace_path: None,
    });

    let result = eframe::run_native(
        "shimmer - psx",
        native_options,
        Box::new(|cc| Ok(Box::new(App::new(cc, cli)))),
    );

    if let Err(e) = result {
        eprintln!("{e:?}");
    }
}
