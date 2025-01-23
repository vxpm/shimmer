use crate::{
    PSX,
    cdrom::{Command, Interpreter, interpreter::Event},
    scheduler,
};
use tinylog::info;

const CDROM_VERSION: [u8; 4] = [0x94, 0x09, 0x19, 0xc0];

const DEFAULT_DELAY: u64 = 50401;
const INIT_ACK_DELAY: u64 = 81102;

impl Interpreter {
    pub fn push_parameter(&mut self, psx: &mut PSX, value: u8) {
        info!(psx.loggers.cdrom, "received parameter {value:#02X}");
        psx.cdrom.parameter_queue.push_back(value);
    }

    pub fn command(&mut self, psx: &mut PSX, value: u8) {
        let cmd = Command::new(value);
        info!(psx.loggers.cdrom, "received command {cmd:?}");

        psx.cdrom.status.set_busy(true);
        match cmd {
            Command::Nop => {
                psx.scheduler
                    .schedule(scheduler::Event::Cdrom(Event::AckGeneric), DEFAULT_DELAY);
            }
            Command::Init => {
                psx.scheduler
                    .schedule(scheduler::Event::Cdrom(Event::AckInit), INIT_ACK_DELAY);

                psx.scheduler.schedule(
                    scheduler::Event::Cdrom(Event::CompleteInit),
                    INIT_ACK_DELAY + DEFAULT_DELAY,
                );
            }
            Command::Demute => {
                psx.scheduler
                    .schedule(scheduler::Event::Cdrom(Event::AckGeneric), DEFAULT_DELAY);
            }
            Command::Test => {
                let param = psx.cdrom.parameter_queue.pop_front().unwrap_or_default();
                if param != 0x20 {
                    todo!()
                }

                psx.cdrom.result_queue.extend(CDROM_VERSION);
                psx.scheduler
                    .schedule(scheduler::Event::Cdrom(Event::AckGeneric), DEFAULT_DELAY);
            }
            _ => todo!("{:?}", cmd),
        }
    }
}
