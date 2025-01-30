use crate::{
    PSX,
    cdrom::{Command, Interpreter, Mode, Sector, interpreter::Event},
    scheduler,
};
use tinylog::info;

pub const CDROM_VERSION: [u8; 4] = [0x94, 0x09, 0x19, 0xc0];
pub const DEFAULT_DELAY: u64 = 50401;
pub const INIT_ACK_DELAY: u64 = 81102;
pub const PAUSE_DELAY: u64 = 7666;
pub const READ_DELAY: u64 = 451021;

impl Interpreter {
    pub fn push_parameter(&mut self, psx: &mut PSX, value: u8) {
        info!(psx.loggers.cdrom, "received parameter {value:#02X}");
        psx.cdrom.parameter_queue.push_back(value);
    }

    pub fn command(&mut self, psx: &mut PSX, value: u8) {
        let cmd = Command::new(value);
        info!(psx.loggers.cdrom, "received command {cmd:?}"; stat = psx.cdrom.status);

        psx.cdrom.command_status.set_busy(true);
        match cmd {
            Command::Nop => {
                psx.scheduler
                    .schedule(scheduler::Event::Cdrom(Event::Acknowledge), DEFAULT_DELAY);
            }
            Command::Init => {
                psx.scheduler
                    .schedule(scheduler::Event::Cdrom(Event::Acknowledge), INIT_ACK_DELAY);
                psx.scheduler.schedule(
                    scheduler::Event::Cdrom(Event::CompleteInit),
                    INIT_ACK_DELAY + DEFAULT_DELAY,
                );
            }
            Command::Demute => {
                psx.scheduler
                    .schedule(scheduler::Event::Cdrom(Event::Acknowledge), DEFAULT_DELAY);
            }
            Command::Test => {
                let param = psx.cdrom.parameter_queue.pop_front().unwrap_or_default();
                if param != 0x20 {
                    todo!()
                }

                psx.cdrom.result_queue.extend(CDROM_VERSION);
                psx.scheduler
                    .schedule(scheduler::Event::Cdrom(Event::Acknowledge), DEFAULT_DELAY);
            }
            Command::GetID => {
                psx.scheduler
                    .schedule(scheduler::Event::Cdrom(Event::Acknowledge), DEFAULT_DELAY);
                psx.scheduler.schedule(
                    scheduler::Event::Cdrom(Event::CompleteGetID),
                    2 * DEFAULT_DELAY,
                );
            }
            Command::SetLocation => {
                let minutes = psx.cdrom.parameter_queue.pop_front().unwrap();
                let seconds = psx.cdrom.parameter_queue.pop_front().unwrap();
                let frames = psx.cdrom.parameter_queue.pop_front().unwrap();
                let decode_bcd = |value| value & 0x0F + 10 * value & (0xF0 >> 4);

                psx.cdrom.location =
                    Sector::new(decode_bcd(minutes), decode_bcd(seconds), decode_bcd(frames));
                psx.scheduler
                    .schedule(scheduler::Event::Cdrom(Event::Acknowledge), DEFAULT_DELAY);
            }
            Command::SetMode => {
                psx.cdrom.mode = Mode::from_bits(psx.cdrom.parameter_queue.pop_front().unwrap());
                info!(psx.loggers.cdrom, "set mode"; mode = psx.cdrom.mode);

                psx.scheduler
                    .schedule(scheduler::Event::Cdrom(Event::Acknowledge), DEFAULT_DELAY);
            }
            Command::ReadN => {
                psx.scheduler
                    .schedule(scheduler::Event::Cdrom(Event::Acknowledge), DEFAULT_DELAY);
                psx.scheduler.schedule(
                    scheduler::Event::Cdrom(Event::Read(true)),
                    DEFAULT_DELAY + READ_DELAY / psx.cdrom.mode.speed().factor(),
                );
            }
            Command::Pause => {
                psx.scheduler
                    .schedule(scheduler::Event::Cdrom(Event::Acknowledge), DEFAULT_DELAY);
                psx.scheduler.schedule(
                    scheduler::Event::Cdrom(Event::CompletePause),
                    DEFAULT_DELAY + PAUSE_DELAY / psx.cdrom.mode.speed().factor(),
                );
            }
            _ => todo!("{:?}", cmd),
        }
    }
}
