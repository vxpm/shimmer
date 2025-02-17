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

    pub fn update(&mut self, joypad: &mut shimmer::sio0::Joypad) {
        while let Some(event) = self.gilrs.next_event() {
            if self.active_gamepad.is_some_and(|id| event.id == id) {
                let digital = &mut joypad.digital_input;
                match event.event {
                    gilrs::EventType::ButtonChanged(button, value, _) => {
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
}
