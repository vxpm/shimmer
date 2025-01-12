use crate::{
    PSX,
    cpu::cop0::Interrupt,
    dma::{Channel, DataDirection, TransferDirection, TransferMode},
    gpu::{self},
    mem::Address,
    scheduler::Event,
};
use bitos::{BitUtils, integer::u24};
use tinylog::{info, warn};

pub enum ExecState {
    None,
    BurstTransfer {
        channel: Channel,
        current_addr: u32,
        remaining: u32,
    },
    SliceTransfer {
        channel: Channel,
    },
    LinkedTransfer {
        channel: Channel,
    },
}

pub struct Executor<'psx> {
    pub psx: &'psx mut PSX,
    pub state: &'psx mut ExecState,
}

impl<'psx> Executor<'psx> {
    pub fn new(psx: &'psx mut PSX, state: &'psx mut ExecState) -> Self {
        Self { psx, state }
    }

    fn check_interrupt(&mut self) {
        if self
            .psx
            .dma
            .interrupt_control
            .update_master_interrupt_flag()
        {
            self.psx.cop0.interrupt_status.request(Interrupt::DMA);
        }
    }

    pub fn advance(&mut self) {
        self.check_interrupt();

        let (channel, finished) = match &mut self.state {
            ExecState::BurstTransfer {
                channel,
                current_addr,
                remaining,
            } => {
                let channel_state = &self.psx.dma.channels[*channel as usize];
                let increment = match channel_state.control.data_direction() {
                    DataDirection::Forward => 4,
                    DataDirection::Backward => -4,
                };

                let finished = match channel {
                    Channel::OTC => {
                        if *remaining > 1 {
                            let prev = current_addr.wrapping_add_signed(increment) & 0x00FF_FFFF;
                            self.psx
                                .write::<_, true>(Address(*current_addr), prev)
                                .unwrap();

                            *remaining -= 1;
                            (*channel, false)
                        } else {
                            self.psx
                                .write::<_, true>(Address(*current_addr), 0x00FF_FFFF)
                                .unwrap();

                            // alt behaviour
                            let channel_state = &mut self.psx.dma.channels[*channel as usize];
                            if channel_state.control.alternative_behaviour() {
                                channel_state.base.set_addr(u24::new(*current_addr));
                                channel_state.block_control.set_len(0);
                            }

                            (*channel, true)
                        }
                    }
                    _ => {
                        warn!(self.psx.loggers.dma, "unimplemented burst transfer");
                        (*channel, true)
                    }
                };

                *current_addr = current_addr.wrapping_add_signed(increment);
                finished
            }
            ExecState::SliceTransfer { channel } => {
                let channel_state = &self.psx.dma.channels[*channel as usize];
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
                    match channel {
                        Channel::GPU => match transfer_direction {
                            TransferDirection::DeviceToRam => todo!(),
                            TransferDirection::RamToDevice => {
                                // assert_eq!(
                                //     self.psx.gpu.status.dma_direction(),
                                //     gpu::DmaDirection::CpuToGp0
                                // );

                                let word =
                                    self.psx.read::<u32, true>(Address(current_addr)).unwrap();
                                self.psx
                                    .gpu
                                    .queue
                                    .enqueue(gpu::cmd::Packet::Rendering(word));
                            }
                        },
                        _ => warn!(self.psx.loggers.dma, "unimplemented slice transfer"),
                    }

                    current_addr = current_addr.wrapping_add_signed(increment);
                }

                // update registers
                let channel_state = &mut self.psx.dma.channels[*channel as usize];
                channel_state.base.set_addr(u24::new(current_addr));
                channel_state.block_control.set_count(count - 1);

                if count > 1 {
                    (*channel, false)
                } else {
                    (*channel, true)
                }
            }
            ExecState::LinkedTransfer { channel } => {
                assert_eq!(*channel, Channel::GPU);

                let channel_status = &self.psx.dma.channels[*channel as usize];
                let current_addr = channel_status.base.addr().value() & !0b11;
                let node = self.psx.read::<u32, true>(Address(current_addr)).unwrap();
                let next = node.bits(0, 24);
                let words = node.bits(24, 32);

                if next == 0x00FF_FFFF {
                    (*channel, true)
                } else {
                    for i in 0..words {
                        let addr = current_addr + (i + 1) * 4;
                        let word = self.psx.read::<u32, true>(Address(addr)).unwrap();
                        self.psx
                            .gpu
                            .queue
                            .enqueue(gpu::cmd::Packet::Rendering(word));
                    }

                    self.psx.dma.channels[*channel as usize]
                        .base
                        .set_addr(u24::new(next & !0b11));

                    (*channel, false)
                }
            }
            ExecState::None => unreachable!(),
        };

        if finished {
            info!(
                self.psx.loggers.dma,
                "finished transfer on channel {channel:?}";
            );
            *self.state = ExecState::None;

            let channel_control = &mut self.psx.dma.channels[channel as usize].control;
            channel_control.set_transfer_ongoing(false);
            channel_control.set_force_transfer(false);

            // set interrupt flag if enabled
            let interrupt_control = &mut self.psx.dma.interrupt_control;
            if interrupt_control
                .channel_interrupt_mask_at(channel as usize)
                .unwrap()
            {
                interrupt_control.set_channel_interrupt_flags_at(channel as usize, true);
            }

            self.check_interrupt();
        } else {
            self.psx
                .scheduler
                .schedule(Event::DmaAdvance, channel.cycles_per_word());
        }
    }

    pub fn update(&mut self) {
        self.check_interrupt();
        match self.state {
            ExecState::None => {
                let mut enabled_channels = self.psx.dma.control.enabled_channels();
                enabled_channels.sort_unstable_by_key(|(_, priority)| std::cmp::Reverse(*priority));

                for (channel, _) in enabled_channels {
                    let channel_state = &self.psx.dma.channels[channel as usize];
                    if channel_state.control.transfer_ongoing() {
                        let dreq = match channel {
                            Channel::OTC => false,
                            Channel::GPU => self.psx.gpu.status.dma_request(),
                            _ => true,
                        };

                        if !dreq && !channel_state.control.force_transfer() {
                            continue;
                        }

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
                                    self.psx.loggers.dma,
                                    "starting burst transfer on channel {channel:?}";
                                    base = Address(current_addr), remaining = remaining
                                );

                                *self.state = ExecState::BurstTransfer {
                                    channel,
                                    current_addr,
                                    remaining,
                                };
                            }
                            TransferMode::Slice => {
                                info!(
                                    self.psx.loggers.dma,
                                    "starting slice transfer on channel {channel:?}";
                                );
                                *self.state = ExecState::SliceTransfer { channel };
                            }
                            TransferMode::LinkedList => {
                                info!(
                                    self.psx.loggers.dma,
                                    "starting linked transfer on channel {channel:?}";
                                );
                                *self.state = ExecState::LinkedTransfer { channel };
                            }
                        }

                        self.psx
                            .scheduler
                            .schedule(Event::DmaAdvance, channel.cycles_per_word());

                        break;
                    }
                }
            }
            _ => (),
        }
    }
}
