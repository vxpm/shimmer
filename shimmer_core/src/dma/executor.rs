use crate::{
    PSX,
    cpu::cop0::Interrupt,
    dma::{Channel, DataDirection, TransferDirection, TransferMode},
    gpu::{self},
    mem::Address,
    scheduler::Event,
};
use bitos::{BitUtils, integer::u24};
use tinylog::{debug, error, info, warn};

enum Progress {
    Ongoing,
    Finished,
}

pub struct BurstTransfer {
    channel: Channel,
    current_addr: u32,
    remaining: u32,
}

impl BurstTransfer {
    fn advance(&mut self, psx: &mut PSX) -> Progress {
        let channel_state = &psx.dma.channels[self.channel as usize];
        let increment = match channel_state.control.data_direction() {
            DataDirection::Forward => 4,
            DataDirection::Backward => -4,
        };

        let progress = match self.channel {
            Channel::OTC => {
                if self.remaining > 1 {
                    let prev = self.current_addr.wrapping_add_signed(increment) & 0x00FF_FFFF;
                    debug!(
                        psx.loggers.dma,
                        "writing {:?} to {}",
                        Address(prev),
                        Address(self.current_addr)
                    );
                    psx.write::<_, true>(Address(self.current_addr), prev)
                        .unwrap();

                    self.remaining -= 1;

                    Progress::Ongoing
                } else {
                    debug!(
                        psx.loggers.dma,
                        "writing {:#08X} to {}",
                        0x00FF_FFFF,
                        Address(self.current_addr)
                    );
                    psx.write::<_, true>(Address(self.current_addr), 0x00FF_FFFF)
                        .unwrap();

                    // alt behaviour
                    let channel_state = &mut psx.dma.channels[self.channel as usize];
                    if channel_state.control.alternative_behaviour() {
                        channel_state.base.set_addr(u24::new(self.current_addr));
                        channel_state.block_control.set_len(0);
                    }

                    Progress::Finished
                }
            }
            _ => {
                error!(psx.loggers.dma, "unimplemented burst transfer");
                Progress::Finished
            }
        };

        self.current_addr = self.current_addr.wrapping_add_signed(increment);
        progress
    }
}

pub struct SliceTransfer {
    channel: Channel,
}

impl SliceTransfer {
    fn advance(&mut self, psx: &mut PSX) -> Progress {
        let channel_state = &psx.dma.channels[self.channel as usize];
        let count = channel_state.block_control.count();
        let len = channel_state.block_control.len();
        let transfer_direction = channel_state.control.transfer_direction();
        let increment = match channel_state.control.data_direction() {
            DataDirection::Forward => 4,
            DataDirection::Backward => -4,
        };

        // perform transfer
        let mut current_addr = channel_state.base.addr().value() & !0b11;
        for _ in 0..len {
            match self.channel {
                Channel::GPU => match transfer_direction {
                    TransferDirection::DeviceToRam => todo!(),
                    TransferDirection::RamToDevice => {
                        if psx.gpu.status.dma_direction() != gpu::DmaDirection::CpuToGp0 {
                            warn!(psx.loggers.gpu, "wrong DMA direction!");
                        }

                        let word = psx.read::<u32, true>(Address(current_addr)).unwrap();
                        psx.gpu.queue.enqueue(gpu::cmd::Packet::Rendering(word));
                    }
                },
                _ => error!(psx.loggers.dma, "unimplemented slice transfer"),
            }

            current_addr = current_addr.wrapping_add_signed(increment);
        }

        // update registers
        let channel_state = &mut psx.dma.channels[self.channel as usize];
        channel_state.base.set_addr(u24::new(current_addr));
        channel_state.block_control.set_count(count - 1);

        if count > 1 {
            Progress::Finished
        } else {
            Progress::Ongoing
        }
    }
}

pub struct LinkedTransfer {
    channel: Channel,
}

impl LinkedTransfer {
    fn advance(&mut self, psx: &mut PSX) -> Progress {
        assert_eq!(self.channel, Channel::GPU);

        let channel_status = &psx.dma.channels[self.channel as usize];
        let current_addr = channel_status.base.addr().value() & !0b11;
        let node = psx.read::<u32, true>(Address(current_addr)).unwrap();
        let next = node.bits(0, 24);
        let words = node.bits(24, 32);

        if next == 0x00FF_FFFF {
            Progress::Finished
        } else {
            for i in 0..words {
                let addr = current_addr + (i + 1) * 4;
                let word = psx.read::<u32, true>(Address(addr)).unwrap();
                psx.gpu.queue.enqueue(gpu::cmd::Packet::Rendering(word));
            }

            psx.dma.channels[self.channel as usize]
                .base
                .set_addr(u24::new(next & !0b11));

            Progress::Ongoing
        }
    }
}

#[derive(Default)]
pub enum Executor {
    #[default]
    Idle,
    BurstTransfer(BurstTransfer),
    SliceTransfer(SliceTransfer),
    LinkedTransfer(LinkedTransfer),
}

fn update_master_interrupt(psx: &mut PSX) {
    if psx.dma.interrupt_control.update_master_interrupt_flag() {
        psx.cop0.interrupt_status.request(Interrupt::DMA);
    }
}

impl Executor {
    #[inline(always)]
    pub fn ongoing(&self) -> bool {
        match self {
            Executor::Idle => false,
            _ => true,
        }
    }

    pub fn advance(&mut self, psx: &mut PSX) {
        update_master_interrupt(psx);

        let (channel, progress) = match self {
            Executor::BurstTransfer(transfer) => (transfer.channel, transfer.advance(psx)),
            Executor::SliceTransfer(transfer) => (transfer.channel, transfer.advance(psx)),
            Executor::LinkedTransfer(transfer) => (transfer.channel, transfer.advance(psx)),
            Executor::Idle => unreachable!(),
        };

        match channel {
            Channel::GPU => {
                psx.scheduler.schedule(Event::Gpu, 0);
            }
            Channel::OTC => (),
            _ => todo!(),
        }

        match progress {
            Progress::Ongoing => {
                psx.scheduler
                    .schedule(Event::DmaAdvance, channel.cycles_per_word());
            }
            Progress::Finished => {
                info!(
                    psx.loggers.dma,
                    "finished transfer on channel {channel:?}";
                );

                *self = Executor::Idle;

                let channel_control = &mut psx.dma.channels[channel as usize].control;
                channel_control.set_transfer_ongoing(false);

                // set interrupt flag if enabled
                let interrupt_control = &mut psx.dma.interrupt_control;
                if interrupt_control
                    .channel_interrupt_mask_at(channel as usize)
                    .unwrap()
                {
                    interrupt_control.set_channel_interrupt_flags_at(channel as usize, true);
                }

                update_master_interrupt(psx);
            }
        }
    }

    pub fn update(&mut self, psx: &mut PSX) {
        update_master_interrupt(psx);

        match self {
            Executor::Idle => {
                let mut enabled_channels = psx.dma.control.enabled_channels();
                enabled_channels.sort_unstable_by_key(|(_, priority)| std::cmp::Reverse(*priority));

                for (channel, _) in enabled_channels {
                    let channel_state = &mut psx.dma.channels[channel as usize];
                    if channel_state.control.transfer_ongoing() {
                        let dreq = match channel {
                            Channel::OTC => false,
                            Channel::GPU => psx.gpu.status.dma_request(),
                            _ => true,
                        };

                        if !dreq && !channel_state.control.force_transfer() {
                            continue;
                        }

                        channel_state.control.set_force_transfer(false);
                        match channel_state
                            .control
                            .transfer_mode()
                            .unwrap_or(TransferMode::Burst)
                        {
                            TransferMode::Burst => {
                                let current_addr = channel_state.base.addr().value() & !0b11;
                                let remaining = if channel_state.block_control.len() == 0 {
                                    0x10000
                                } else {
                                    u32::from(channel_state.block_control.len())
                                };

                                info!(
                                    psx.loggers.dma,
                                    "starting burst transfer on channel {channel:?}";
                                    base = Address(current_addr), remaining = remaining
                                );

                                *self = Executor::BurstTransfer(BurstTransfer {
                                    channel,
                                    current_addr,
                                    remaining,
                                });
                            }
                            TransferMode::Slice => {
                                info!(
                                    psx.loggers.dma,
                                    "starting slice transfer on channel {channel:?}";
                                );

                                *self = Executor::SliceTransfer(SliceTransfer { channel });
                            }
                            TransferMode::LinkedList => {
                                info!(
                                    psx.loggers.dma,
                                    "starting linked transfer on channel {channel:?}";
                                );

                                *self = Executor::LinkedTransfer(LinkedTransfer { channel });
                            }
                        }

                        psx.scheduler
                            .schedule(Event::DmaAdvance, channel.cycles_per_word());
                        return;
                    }
                }
            }
            _ => (),
        }
    }
}
