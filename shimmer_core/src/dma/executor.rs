use crate::{
    PSX,
    cpu::cop0::Interrupt,
    dma::{Channel, TransferMode},
    gpu,
    mem::Address,
};
use bitos::BitUtils;
use tinylog::{debug, info};

pub struct Executor<'psx> {
    pub psx: &'psx mut PSX,
}

impl<'psx> Executor<'psx> {
    pub fn new(psx: &'psx mut PSX) -> Self {
        Self { psx }
    }

    fn transfer_burst(&mut self, channel: Channel) {
        let channel_base = &self.psx.dma.channels[channel as usize].base;
        let channel_block_control = &self.psx.dma.channels[channel as usize].block_control;

        match channel {
            Channel::OTC => {
                let base = channel_base.addr().value() & !0b11;
                let entries = if channel_block_control.len() == 0 {
                    0x10000
                } else {
                    u32::from(channel_block_control.len())
                };

                debug!(
                    self.psx.loggers.dma,
                    "base = {}, entries = {entries}",
                    Address(base)
                );

                let mut addr = base;
                for _ in 1..entries {
                    let prev = addr.wrapping_sub(4) & 0x00FF_FFFF;
                    self.psx.write::<_, true>(Address(addr), prev).unwrap();

                    let region = Address(addr).physical().and_then(|p| p.region());
                    debug!(
                        self.psx.loggers.dma,
                        "[{}] = {} ({region:?})",
                        Address(addr),
                        Address(prev)
                    );

                    addr = prev;
                }

                self.psx
                    .write::<_, true>(Address(addr), 0x00FF_FFFF)
                    .unwrap();
                debug!(self.psx.loggers.dma, "[{}] = 0x00FF_FFFF", Address(addr));
                debug!(
                    self.psx.loggers.dma,
                    "FINISHED - addr: {}",
                    self.psx.cpu.instr_delay_slot().1
                );
            }
            _ => todo!(),
        }
    }

    fn transfer_slice(&mut self, channel: Channel) {
        match channel {
            Channel::OTC => self.transfer_burst(channel),
            _ => todo!(),
        }
    }

    fn transfer_linked(&mut self, channel: Channel) {
        let channel_base = &self.psx.dma.channels[channel as usize].base;

        match channel {
            Channel::OTC => self.transfer_burst(channel),
            Channel::GPU => {
                let mut current = channel_base.addr().value() & !0b11;
                loop {
                    let node = self.psx.read::<u32, true>(Address(current)).unwrap();
                    let next = node.bits(0, 24);
                    let words = node.bits(24, 32);

                    if next == 0x00FF_FFFF {
                        break;
                    }

                    for i in 0..words {
                        let addr = current + (i + 1) * 4;
                        let word = self.psx.read::<u32, true>(Address(addr)).unwrap();
                        self.psx
                            .gpu
                            .queue
                            .push_back(gpu::instr::Instruction::Rendering(
                                gpu::instr::RenderingInstruction::from_bits(word),
                            ));
                    }

                    current = next & !0b11;
                }
            }
            _ => todo!(),
        }
    }

    pub fn progress_transfers(&mut self) {
        let mut enabled_channels = self.psx.dma.control.enabled_channels();
        enabled_channels.sort_unstable_by_key(|(_, priority)| std::cmp::Reverse(*priority));

        for (channel, _) in enabled_channels {
            let channel_control = &self.psx.dma.channels[channel as usize].control;
            if channel_control.transfer_ongoing() {
                info!(self.psx.loggers.dma, "{channel:?} ongoing"; control = channel_control.clone());

                match channel_control
                    .transfer_mode()
                    .unwrap_or(TransferMode::Burst)
                {
                    TransferMode::Burst => self.transfer_burst(channel),
                    TransferMode::Slice => self.transfer_slice(channel),
                    TransferMode::LinkedList => self.transfer_linked(channel),
                }

                let channel_control = &mut self.psx.dma.channels[channel as usize].control;
                channel_control.set_transfer_ongoing(false);
                channel_control.set_force_transfer(false);

                let interrupt_control = &mut self.psx.dma.interrupt_control;
                if interrupt_control
                    .channel_interrupt_mask_at(channel as usize)
                    .unwrap()
                {
                    interrupt_control.set_channel_interrupt_flags_at(channel as usize, true);
                }

                let old_master_interrupt = interrupt_control.master_interrupt_flag();
                let new_master_interrupt = interrupt_control.bus_error()
                    || (interrupt_control.master_channel_interrupt_enable()
                        && interrupt_control.channel_interrupt_flags_raw().value() != 0);

                interrupt_control.set_master_interrupt_flag(new_master_interrupt);

                if !old_master_interrupt && new_master_interrupt {
                    self.psx.cop0.interrupt_status.request(Interrupt::DMA);
                }
            }
        }
    }
}
