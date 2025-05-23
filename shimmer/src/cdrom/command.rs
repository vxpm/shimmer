use super::Event;
use crate::{
    PSX,
    cdrom::{Cdrom, Command},
    scheduler,
};
use tinylog::trace;

pub const ACK_DEFAULT_DELAY: u64 = 50401;
pub const ACK_INIT_DELAY: u64 = 81102;

impl Cdrom {
    pub fn push_parameter(&mut self, psx: &mut PSX, value: u8) {
        trace!(psx.loggers.cdrom, "received parameter {value:#02X}");
        psx.cdrom.parameter_queue.push_back(value);
    }

    pub fn command(&mut self, psx: &mut PSX, value: u8) {
        psx.cdrom.command_status.set_busy(true);

        let cmd = Command::new(value);
        // info!(psx.loggers.cdrom, "received command {cmd:?}"; stat = psx.cdrom.status);

        let delay = match cmd {
            // Command::Nop
            // | Command::Test
            // | Command::Mute
            // | Command::Demute
            // | Command::GetID
            // | Command::SetLocation
            // | Command::SetMode
            // | Command::ReadN
            // | Command::ReadS
            // | Command::Pause
            // | Command::SeekL
            // | Command::SetFilter
            // | Command::GetTN
            // | Command::GetTD
            // | Command::GetLocationP => ACK_DEFAULT_DELAY,
            Command::Init => ACK_INIT_DELAY,
            _ => ACK_DEFAULT_DELAY,
        };

        psx.scheduler
            .schedule(scheduler::Event::Cdrom(Event::Acknowledge(cmd)), delay);
    }
}
