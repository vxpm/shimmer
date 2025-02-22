use eframe::egui::{Context, Key};
use gilrs::{Button, GamepadId, Gilrs};

pub struct Input {
    gilrs: Gilrs,
    active_gamepad: Option<GamepadId>,
}

impl Input {
    pub fn new() -> Self {
        let gilrs = Gilrs::new().unwrap();
        let active_gamepad = gilrs.gamepads().next().map(|(id, _)| id);

        Self {
            gilrs,
            active_gamepad,
        }
    }

    pub fn update(&mut self, ctx: &Context, joypad: &mut shimmer::sio0::Joypad) {
        if self.active_gamepad.is_none() {
            ctx.input(|i| {
                let digital = &mut joypad.digital_input;
                digital.set_cross(i.key_down(Key::X));
                digital.set_square(i.key_down(Key::Z));
                digital.set_circle(i.key_down(Key::C));
                digital.set_triangle(i.key_down(Key::V));

                digital.set_joy_right(i.key_down(Key::ArrowRight));
                digital.set_joy_left(i.key_down(Key::ArrowLeft));
                digital.set_joy_up(i.key_down(Key::ArrowUp));
                digital.set_joy_down(i.key_down(Key::ArrowDown));

                digital.set_start(i.key_down(Key::Space));
                digital.set_select(i.key_down(Key::Q));

                if i.key_down(Key::W) {
                    digital.set_start(!digital.start());
                }
            });
        }

        while let Some(event) = self.gilrs.next_event() {
            match event.event {
                gilrs::EventType::ButtonChanged(button, value, _)
                    if self.active_gamepad.is_some_and(|id| event.id == id) =>
                {
                    let digital = &mut joypad.digital_input;
                    let level = value > 0.0;
                    match button {
                        Button::South => {
                            digital.set_cross(level);
                        }
                        Button::East => {
                            digital.set_circle(level);
                        }
                        Button::North => {
                            digital.set_triangle(level);
                        }
                        Button::West => {
                            digital.set_square(level);
                        }
                        Button::LeftTrigger => {
                            digital.set_l1(level);
                        }
                        Button::LeftTrigger2 => {
                            digital.set_l2(level);
                        }
                        Button::RightTrigger => {
                            digital.set_r1(level);
                        }
                        Button::RightTrigger2 => {
                            digital.set_r2(level);
                        }
                        Button::Select => {
                            digital.set_select(level);
                            digital.set_start(!digital.start());
                        }
                        Button::Start => {
                            digital.set_start(level);
                        }
                        Button::LeftThumb => {
                            digital.set_l3(level);
                        }
                        Button::RightThumb => {
                            digital.set_r3(level);
                        }
                        Button::DPadUp => {
                            digital.set_joy_up(level);
                        }
                        Button::DPadDown => {
                            digital.set_joy_down(level);
                        }
                        Button::DPadLeft => {
                            digital.set_joy_left(level);
                        }
                        Button::DPadRight => {
                            digital.set_joy_right(level);
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
}
