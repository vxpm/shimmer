use crate::{
    PSX,
    cdrom::{Interpreter, InterruptMask, InterruptStatus},
};
use bitos::{bitos, integer::u3};
use tinylog::info;

#[bitos(8)]
struct InterruptFlags {
    #[bits(0..3)]
    acknowledge_interrupt_bits: u3,
    #[bits(3)]
    acknowledge_sound_buffer_empty: bool,
    #[bits(4)]
    acknowledge_sound_buffer_write_ready: bool,
    #[bits(5)]
    clear_sound_buffer: bool,
    #[bits(6)]
    clear_parameter_fifo: bool,
    #[bits(7)]
    reset_decoder: bool,
}

impl Interpreter {
    pub fn set_interrupt_mask(&mut self, psx: &mut PSX, value: u8) {
        psx.cdrom.interrupt_mask = InterruptMask::from_bits(value);
    }

    pub fn ack_interrupt_status(&mut self, psx: &mut PSX, value: u8) {
        info!(psx.loggers.cdrom, "acknowledging interrupts status");
        let cmd = InterruptFlags::from_bits(value);

        let status = psx.cdrom.interrupt_status.to_bits();
        let new_status = status & !value;
        psx.cdrom.interrupt_status = InterruptStatus::from_bits(new_status | 0b1110_0000);

        if cmd.clear_sound_buffer() {
            todo!("clear sound buffer");
        }

        if cmd.clear_parameter_fifo() {
            psx.cdrom.parameter_queue.clear();
        }

        if cmd.reset_decoder() {
            todo!("reset decoder");
        }
    }
}
