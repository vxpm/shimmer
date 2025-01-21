use shimmer_cli::State;
use shimmer_core::Emulator;
use tinylog::Logger;
use winit::{
    dpi::LogicalSize,
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::WindowBuilder,
};

async fn run() {
    env_logger::init();

    let (mut emulator, receiver) = Emulator::with_bios(
        std::fs::read("resources/BIOS.BIN").unwrap(),
        Logger::dummy(),
    );

    std::thread::Builder::new()
        .name("emulator thread".to_owned())
        .spawn(move || {
            loop {
                emulator.cycle();
            }
        })
        .unwrap();

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(640 * 2, 480 * 2))
        .build(&event_loop)
        .unwrap();
    let mut state = State::new(&window, receiver).await;

    event_loop
        .run(|event, control_flow| match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => match event {
                WindowEvent::RedrawRequested => {
                    state.window().request_redraw();
                    state.update();

                    match state.render() {
                        Ok(_) => {}

                        // Reconfigure the surface if it's lost or outdated
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            state.resize(state.size)
                        }
                        // The system is out of memory, we should probably quit
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            eprintln!("OutOfMemory");
                            control_flow.exit();
                        }

                        // This happens when the a frame takes too long to present
                        Err(wgpu::SurfaceError::Timeout) => {
                            eprintln!("Surface timeout")
                        }
                    }
                }
                WindowEvent::Resized(physical_size) => {
                    state.resize(*physical_size);
                }
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            state: ElementState::Pressed,
                            physical_key: PhysicalKey::Code(KeyCode::Escape),
                            ..
                        },
                    ..
                } => control_flow.exit(),
                _ => {}
            },
            _ => {}
        })
        .unwrap();
}

fn main() {
    pollster::block_on(run());
}
