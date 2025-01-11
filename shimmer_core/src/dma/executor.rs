use crate::{
    PSX,
    cpu::cop0::Interrupt,
    dma::{Channel, DataDirection, TransferDirection, TransferMode},
    gpu::{self},
    mem::Address,
};
use bitos::{BitUtils, integer::u24};
use tinylog::info;

pub struct Executor<'psx> {
    pub psx: &'psx mut PSX,
}

impl<'psx> Executor<'psx> {
    pub fn new(psx: &'psx mut PSX) -> Self {
        Self { psx }
    }

    fn transfer_burst(&mut self, channel: Channel) {
        let channel_state = &self.psx.dma.channels[channel as usize];

        info!(self.psx.loggers.dma, "BURST TRANSFER OF {channel:?}");
        let base = channel_state.base.addr().value() & !0b11;
        let entries = if channel_state.block_control.len() == 0 {
            0x10000
        } else {
            u32::from(channel_state.block_control.len())
        };
        let increment = match channel_state.control.data_direction() {
            DataDirection::Forward => 4,
            DataDirection::Backward => -4,
        };

        // perform transfer
        let mut current = base;
        for index in 0..entries {
            match channel {
                Channel::OTC => {
                    if index + 1 == entries {
                        self.psx
                            .write::<_, true>(Address(current), 0x00FF_FFFF)
                            .unwrap();
                    } else {
                        let prev = current.wrapping_sub(4) & 0x00FF_FFFF;
                        self.psx.write::<_, true>(Address(current), prev).unwrap();
                    }
                }
                _ => todo!(),
            }

            current = current.wrapping_add_signed(increment);
        }

        // alt behaviour
        if self.psx.dma.channels[channel as usize]
            .control
            .alternative_behaviour()
        {
            self.psx.dma.channels[channel as usize]
                .base
                .set_addr(u24::new(current));

            self.psx.dma.channels[channel as usize]
                .block_control
                .set_len(0);
        }
    }

    fn transfer_slice(&mut self, channel: Channel) {
        if channel == Channel::OTC {
            self.transfer_burst(channel);
            return;
        }

        info!(self.psx.loggers.dma, "SLICE TRANSFER OF {channel:?}");
        let channel_state = &self.psx.dma.channels[channel as usize];

        let count = channel_state.block_control.count();
        let len = channel_state.block_control.len();
        let total = count as u32 * len as u32;

        let transfer_direction = channel_state.control.transfer_direction();
        let increment = match channel_state.control.data_direction() {
            DataDirection::Forward => 4,
            DataDirection::Backward => -4,
        };

        // perform transfer
        let mut current = channel_state.base.addr().value();
        for _ in 0..total {
            match channel {
                Channel::GPU => match transfer_direction {
                    TransferDirection::DeviceToRam => todo!(),
                    TransferDirection::RamToDevice => {
                        assert_eq!(
                            self.psx.gpu.status.dma_direction(),
                            gpu::DmaDirection::CpuToGp0
                        );

                        let word = self.psx.read::<u32, true>(Address(current)).unwrap();
                        self.psx
                            .gpu
                            .queue
                            .enqueue(gpu::cmd::Packet::Rendering(word));
                    }
                },
                _ => todo!(),
            }

            current = current.wrapping_add_signed(increment);
        }

        self.psx.dma.channels[channel as usize]
            .base
            .set_addr(u24::new(current));

        self.psx.dma.channels[channel as usize]
            .block_control
            .set_count(0);
    }

    fn transfer_linked(&mut self, channel: Channel) {
        if channel == Channel::OTC {
            self.transfer_burst(channel);
            return;
        }

        info!(self.psx.loggers.dma, "LINKED TRANSFER OF {channel:?}");
        assert_eq!(channel, Channel::GPU);

        let channel_status = &self.psx.dma.channels[channel as usize];
        let mut current = channel_status.base.addr().value() & !0b11;
        loop {
            let node = self.psx.read::<u32, true>(Address(current)).unwrap();
            let next = node.bits(0, 24);
            let words = node.bits(24, 32);

            if next == 0x00FF_FFFF {
                self.psx.dma.channels[channel as usize]
                    .base
                    .set_addr(u24::new(current));

                break;
            }

            for i in 0..words {
                let addr = current + (i + 1) * 4;
                let word = self.psx.read::<u32, true>(Address(addr)).unwrap();
                self.psx
                    .gpu
                    .queue
                    .enqueue(gpu::cmd::Packet::Rendering(word));
            }

            current = next & !0b11;
        }
    }

    pub fn progress_transfers(&mut self) {
        let mut enabled_channels = self.psx.dma.control.enabled_channels();
        enabled_channels.sort_unstable_by_key(|(_, priority)| std::cmp::Reverse(*priority));

        for (channel, _) in enabled_channels {
            let channel_control = &self.psx.dma.channels[channel as usize].control;
            if channel_control.transfer_ongoing() {
                if channel == Channel::OTC && !channel_control.force_transfer() {
                    continue;
                }

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
