use crate::{PSX, cdrom::Interpreter};
use bitos::bitos;
use tinylog::debug;

#[bitos(8)]
struct ControlRequest {
    #[bits(5)]
    sound_map_enable: bool,
    #[bits(6)]
    request_sector_buffer_write: bool,
    #[bits(7)]
    request_sector_buffer_read: bool,
}

impl Interpreter {
    pub fn control_request(&mut self, psx: &mut PSX, value: u8) {
        debug!(psx.loggers.cdrom, "control request");
        let cmd = ControlRequest::from_bits(value);

        if cmd.request_sector_buffer_write() {
            debug!(psx.loggers.cdrom, "prepare for writes to WRDATA");
        }

        if cmd.request_sector_buffer_read() {
            debug!(psx.loggers.cdrom, "prepare for reads from RDDATA");
        }
    }
}
