use crate::{
    PSX,
    cdrom::{Command, Interpreter, InterruptKind, Mode},
};
use tinylog::info;

const CDROM_VERSION: [u8; 4] = [0x94, 0x09, 0x19, 0xc0];

impl Interpreter {
    pub fn push_parameter(&mut self, psx: &mut PSX, value: u8) {
        psx.cdrom.parameter_queue.push_back(value);
    }

    pub fn command(&mut self, psx: &mut PSX, value: u8) {
        let cmd = Command::new(value);
        info!(psx.loggers.cdrom, "received command {cmd:?}");

        match cmd {
            Command::Nop => {
                psx.cdrom.result_queue.push_back(psx.cdrom.status.to_bits());
                psx.cdrom.set_interrupt_kind(InterruptKind::Acknowledge);
            }
            Command::Init => {
                psx.cdrom.mode = Mode::from_bits(0x20);
                psx.cdrom.result_queue.push_back(psx.cdrom.status.to_bits());
                psx.cdrom.result_queue.push_back(psx.cdrom.status.to_bits());

                // TODO: activate drive motor? standby? abort all commands???
                psx.cdrom.set_interrupt_kind(InterruptKind::Complete);
            }
            Command::Demute => {
                psx.cdrom.result_queue.push_back(psx.cdrom.status.to_bits());

                psx.cdrom.set_interrupt_kind(InterruptKind::Acknowledge);
            }
            Command::Test => {
                let param = psx.cdrom.parameter_queue.pop_back().unwrap_or_default();
                if param != 0x20 {
                    todo!()
                }

                psx.cdrom.result_queue.extend(CDROM_VERSION);
                psx.cdrom.set_interrupt_kind(InterruptKind::Acknowledge);
            }
            _ => todo!("{:?}", cmd),
        }
    }
}
