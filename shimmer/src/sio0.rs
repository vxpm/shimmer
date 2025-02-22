use crate::{PSX, scheduler};
use shimmer_core::{
    CYCLES_MICROS, Cycles,
    interrupts::Interrupt,
    sio0::{AnalogInput, DigitalInput},
};
use tinylog::{debug, trace};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Update,
    Transfer,
    StartAck,
    EndAck,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoypadCommand {
    Read { change_mode: bool },
    SetLed,
    GetLed,
}

#[derive(Debug, Clone, Copy, Default)]
enum State {
    #[default]
    Idle,
    JoypadStart,
    JoypadTransfer {
        command: JoypadCommand,
        stage: u8,
    },
}

#[derive(Debug, Clone, Default)]
pub struct Joypad {
    pub digital_input: DigitalInput,
    pub analog_left: AnalogInput,
    pub analog_right: AnalogInput,
}

#[derive(Debug, Clone, Default)]
pub struct Sio0 {
    state: State,
    in_progress: bool,

    joypad: Joypad,
    analog_mode: bool,
    config_mode: bool,
}

const TRANSFER_DELAY: Cycles = 46 * CYCLES_MICROS;
const START_ACK_DELAY: Cycles = 3 * CYCLES_MICROS;
const END_ACK_DELAY: Cycles = 2 * CYCLES_MICROS;

impl Sio0 {
    fn update_status(&mut self, psx: &mut PSX) {
        psx.sio0.status.set_tx_ready(psx.sio0.tx.is_none());
        psx.sio0.status.set_rx_ready(psx.sio0.rx.is_some());
        psx.sio0
            .status
            .set_tx_finished(psx.sio0.tx.is_none() && !self.in_progress);
    }

    fn can_transfer(&mut self, psx: &mut PSX) -> bool {
        psx.sio0.control.selected()
            && psx.sio0.control.tx_enable()
            && psx.sio0.tx.is_some()
            && !self.in_progress
    }

    pub fn update(&mut self, psx: &mut PSX, event: Event) {
        self.update_status(psx);

        if psx.sio0.control.acknowledge() {
            psx.sio0.control.set_acknowledge(false);
            psx.sio0.status.set_interrupt_request(false);
        }

        // do something
        match (&mut self.state, event) {
            (_, Event::Update) => {
                // check if a transfer should start
                if self.can_transfer(psx) {
                    self.in_progress = true;
                    psx.scheduler
                        .schedule(scheduler::Event::Sio(Event::Transfer), TRANSFER_DELAY);
                }
            }
            (_, Event::StartAck) => {
                trace!(psx.loggers.sio, "start ack");
                psx.sio0.status.set_device_ready_to_receive(true);
                psx.scheduler
                    .schedule(scheduler::Event::Sio(Event::EndAck), END_ACK_DELAY);

                if psx.sio0.control.device_ready_to_receive_interrupt_enable() {
                    psx.sio0.status.set_interrupt_request(true);
                    psx.interrupts
                        .status
                        .request(Interrupt::ControllerAndMemCard);
                }
            }
            (_, Event::EndAck) => {
                trace!(psx.loggers.sio, "end ack");
                psx.sio0.status.set_device_ready_to_receive(false);
            }
            (State::Idle, Event::Transfer) => {
                self.in_progress = false;
                psx.sio0.rx = Some(0xFF);

                let address = psx.sio0.tx.take().unwrap();
                match address {
                    0x01 if !psx.sio0.control.port_select() => {
                        psx.scheduler
                            .schedule(scheduler::Event::Sio(Event::StartAck), START_ACK_DELAY);
                        self.state = State::JoypadStart;
                    }
                    _ => {}
                }
            }
            (State::JoypadStart, Event::Transfer) => {
                self.in_progress = false;

                debug!(psx.loggers.sio, "joypad start - sending ID");
                psx.sio0.rx = Some(match (self.config_mode, self.analog_mode) {
                    (true, _) => 0xF3,
                    (_, true) => 0x73,
                    (_, false) => 0x41,
                });

                let command = psx.sio0.tx.take().unwrap();
                let command = match command {
                    0x42 => JoypadCommand::Read { change_mode: false },
                    0x43 => JoypadCommand::Read { change_mode: true },
                    0x44 => JoypadCommand::SetLed,
                    0x45 => JoypadCommand::GetLed,
                    _ => todo!("unknown command: {command}"),
                };

                psx.scheduler
                    .schedule(scheduler::Event::Sio(Event::StartAck), START_ACK_DELAY);
                self.state = State::JoypadTransfer { command, stage: 0 };
            }
            (
                State::JoypadTransfer {
                    command: JoypadCommand::Read { change_mode },
                    stage,
                },
                Event::Transfer,
            ) => 'block: {
                self.in_progress = false;

                let data = psx.sio0.tx.take().unwrap();
                match stage {
                    0 => {
                        debug!(psx.loggers.sio, "sending fixed");

                        if *change_mode {
                            match data {
                                0 => self.config_mode = false,
                                1 => self.config_mode = true,
                                _ => panic!("unknown mode"),
                            }
                        } else {
                            assert_eq!(data, 0);
                        }

                        psx.sio0.rx = Some(0x5A);
                        psx.scheduler
                            .schedule(scheduler::Event::Sio(Event::StartAck), START_ACK_DELAY);
                    }
                    1 => {
                        debug!(psx.loggers.sio, "sending switches low");
                        psx.sio0.rx = Some(!self.joypad.digital_input.to_bits().to_le_bytes()[0]);
                        psx.scheduler
                            .schedule(scheduler::Event::Sio(Event::StartAck), START_ACK_DELAY);
                    }
                    2 => {
                        debug!(psx.loggers.sio, "sending switches high");
                        psx.sio0.rx = Some(!self.joypad.digital_input.to_bits().to_le_bytes()[1]);

                        if self.analog_mode || self.config_mode {
                            psx.scheduler
                                .schedule(scheduler::Event::Sio(Event::StartAck), START_ACK_DELAY);
                        } else {
                            self.state = State::Idle;
                            break 'block;
                        }
                    }
                    3 => {
                        debug!(psx.loggers.sio, "sending right analog x");
                        assert_eq!(data, 0x00);

                        psx.sio0.rx = Some(!self.joypad.analog_right.analog_x());
                        psx.scheduler
                            .schedule(scheduler::Event::Sio(Event::StartAck), START_ACK_DELAY);
                    }
                    4 => {
                        debug!(psx.loggers.sio, "sending right analog y");
                        assert_eq!(data, 0x00);

                        psx.sio0.rx = Some(!self.joypad.analog_right.analog_y());
                        psx.scheduler
                            .schedule(scheduler::Event::Sio(Event::StartAck), START_ACK_DELAY);
                    }
                    5 => {
                        debug!(psx.loggers.sio, "sending left analog x");
                        assert_eq!(data, 0x00);

                        psx.sio0.rx = Some(!self.joypad.analog_left.analog_x());
                        psx.scheduler
                            .schedule(scheduler::Event::Sio(Event::StartAck), START_ACK_DELAY);
                    }
                    6 => {
                        debug!(psx.loggers.sio, "sending left analog y");
                        assert_eq!(data, 0x00);

                        psx.sio0.rx = Some(!self.joypad.analog_left.analog_y());
                        self.state = State::Idle;
                        break 'block;
                    }
                    _ => unreachable!(),
                }

                *stage += 1;
            }
            (
                State::JoypadTransfer {
                    command: JoypadCommand::SetLed,
                    stage,
                },
                Event::Transfer,
            ) => 'block: {
                self.in_progress = false;

                let data = psx.sio0.tx.take().unwrap();
                match stage {
                    0 => {
                        debug!(psx.loggers.sio, "sending fixed");
                        assert_eq!(data, 0);

                        psx.sio0.rx = Some(0x5A);
                        psx.scheduler
                            .schedule(scheduler::Event::Sio(Event::StartAck), START_ACK_DELAY);
                    }
                    1 => {
                        debug!(psx.loggers.sio, "sending empty 0 (led)");
                        self.analog_mode = data == 1;

                        psx.sio0.rx = Some(0x00);
                        psx.scheduler
                            .schedule(scheduler::Event::Sio(Event::StartAck), START_ACK_DELAY);
                    }
                    2 => {
                        debug!(psx.loggers.sio, "sending empty 1 (key)");
                        psx.sio0.rx = Some(0x00);

                        psx.scheduler
                            .schedule(scheduler::Event::Sio(Event::StartAck), START_ACK_DELAY);
                    }
                    3 | 4 | 5 | 6 => {
                        debug!(psx.loggers.sio, "sending empty {}", *stage - 1);
                        assert_eq!(data, 0x00);

                        psx.sio0.rx = Some(0x00);

                        if *stage == 6 {
                            self.state = State::Idle;
                            break 'block;
                        } else {
                            psx.scheduler
                                .schedule(scheduler::Event::Sio(Event::StartAck), START_ACK_DELAY);
                        }
                    }
                    _ => unreachable!(),
                }

                *stage += 1;
            }
            (
                State::JoypadTransfer {
                    command: JoypadCommand::GetLed,
                    stage,
                },
                Event::Transfer,
            ) => 'block: {
                self.in_progress = false;

                let data = psx.sio0.tx.take().unwrap();
                match stage {
                    0 => {
                        debug!(psx.loggers.sio, "sending fixed");
                        assert_eq!(data, 0);

                        psx.sio0.rx = Some(0x5A);
                        psx.scheduler
                            .schedule(scheduler::Event::Sio(Event::StartAck), START_ACK_DELAY);
                    }
                    1 => {
                        debug!(psx.loggers.sio, "sending type");

                        psx.sio0.rx = Some(0x01);
                        psx.scheduler
                            .schedule(scheduler::Event::Sio(Event::StartAck), START_ACK_DELAY);
                    }
                    2 => {
                        debug!(psx.loggers.sio, "sending padding");

                        psx.sio0.rx = Some(0x02);
                        psx.scheduler
                            .schedule(scheduler::Event::Sio(Event::StartAck), START_ACK_DELAY);
                    }
                    3 => {
                        debug!(psx.loggers.sio, "sending led");

                        psx.sio0.rx = Some(self.analog_mode as u8);
                        psx.scheduler
                            .schedule(scheduler::Event::Sio(Event::StartAck), START_ACK_DELAY);
                    }
                    4 | 5 | 6 => {
                        debug!(psx.loggers.sio, "sending empty {}", *stage - 4);

                        psx.sio0.rx = Some(6 - *stage);

                        if *stage == 6 {
                            self.state = State::Idle;
                            break 'block;
                        } else {
                            psx.scheduler
                                .schedule(scheduler::Event::Sio(Event::StartAck), START_ACK_DELAY);
                        }
                    }
                    _ => unreachable!(),
                }

                *stage += 1;
            }
        }

        self.update_status(psx);
    }

    pub fn joypad_mut(&mut self) -> &mut Joypad {
        &mut self.joypad
    }
}
