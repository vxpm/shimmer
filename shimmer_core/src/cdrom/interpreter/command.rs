use crate::{
    PSX,
    cdrom::{Command, Interpreter, interpreter::Event},
    scheduler,
};
use tinylog::trace;

pub const ACK_DEFAULT_DELAY: u64 = 50401;
pub const ACK_INIT_DELAY: u64 = 81102;

impl Interpreter {
    pub fn push_parameter(&mut self, psx: &mut PSX, value: u8) {
        trace!(psx.loggers.cdrom, "received parameter {value:#02X}");
        psx.cdrom.parameter_queue.push_back(value);
    }

    pub fn command(&mut self, psx: &mut PSX, value: u8) {
        psx.cdrom.command_status.set_busy(true);

        let cmd = Command::new(value);
        // info!(psx.loggers.cdrom, "received command {cmd:?}"; stat = psx.cdrom.status);

        let delay = match cmd {
            Command::Nop
            | Command::Test
            | Command::Mute
            | Command::Demute
            | Command::GetID
            | Command::SetLocation
            | Command::SetMode
            | Command::ReadN
            | Command::ReadS
            | Command::Pause
            | Command::SeekL
            | Command::SetFilter => ACK_DEFAULT_DELAY,
            Command::Init => ACK_INIT_DELAY,
            _ => todo!("schedule {cmd:?}"),
        };

        psx.scheduler
            .schedule(scheduler::Event::Cdrom(Event::Acknowledge(cmd)), delay);
    }
}
