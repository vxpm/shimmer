use crate::{PSX, cdrom::Cdrom};
use bitos::bitos;
use tinylog::trace;

#[bitos(8)]
#[derive(Debug, Clone, Copy)]
struct ControlRequest {
    #[bits(5)]
    sound_map_enable: bool,
    #[bits(6)]
    request_sector_buffer_write: bool,
    #[bits(7)]
    request_sector_buffer_read: bool,
}

impl Cdrom {
    pub fn control_request(&mut self, psx: &mut PSX, value: u8) {
        let cmd = ControlRequest::from_bits(value);
        trace!(psx.loggers.cdrom, "control request"; request = cmd);
        psx.cdrom.lock_sector_data = !cmd.request_sector_buffer_read();
    }
}
