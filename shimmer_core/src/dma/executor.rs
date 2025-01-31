//! An executor for DMA transfers.

use crate::{
    PSX, cdrom,
    dma::{Channel, DataDirection, TransferDirection, TransferMode},
    interrupts::Interrupt,
    mem::Address,
    scheduler::Event,
};
use bitos::{BitUtils, integer::u24};
use tinylog::{error, info, trace, warn};

/// The progress made by a transfer.
enum Progress {
    /// The transfer is still ongoing.
    Ongoing,
    /// The transfer has yielded control of the bus back to the CPU.
    Yielded,
    /// The transfer has finished.
    Finished,
}

/// An ongoing burst transfer.
struct BurstTransfer {
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
                    psx.write::<_, true>(Address(self.current_addr), prev)
                        .unwrap();

                    self.remaining -= 1;

                    Progress::Ongoing
                } else {
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
            Channel::CDROM => {
                let data = if psx.cdrom.lock_data_queue {
                    [0; 4]
                } else {
                    [
                        psx.cdrom.read_from_sector(),
                        psx.cdrom.read_from_sector(),
                        psx.cdrom.read_from_sector(),
                        psx.cdrom.read_from_sector(),
                    ]
                };

                psx.write::<_, true>(Address(self.current_addr), u32::from_le_bytes(data))
                    .unwrap();

                self.remaining -= 1;
                if self.remaining == 0 {
                    // alt behaviour
                    let channel_state = &mut psx.dma.channels[self.channel as usize];
                    if channel_state.control.alternative_behaviour() {
                        channel_state.base.set_addr(u24::new(self.current_addr));
                        channel_state.block_control.set_len(0);
                    }

                    psx.scheduler
                        .schedule(Event::Cdrom(cdrom::Event::Update), 0);

                    Progress::Finished
                } else {
                    Progress::Ongoing
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

/// An ongoing slice transfer.
struct SliceTransfer {
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
                    TransferDirection::DeviceToRam => {
                        let value = psx.gpu.response_queue.pop_front().unwrap();
                        psx.write::<u32, true>(Address(current_addr), value)
                            .unwrap();
                    }
                    TransferDirection::RamToDevice => {
                        let word = psx.read::<u32, true>(Address(current_addr)).unwrap();
                        psx.gpu.render_queue.push_back(word);
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
            Progress::Yielded
        } else {
            Progress::Finished
        }
    }
}

/// An ongoing linked list transfer.
struct LinkedTransfer {
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
                psx.gpu.render_queue.push_back(word);
            }

            psx.dma.channels[self.channel as usize]
                .base
                .set_addr(u24::new(next & !0b11));

            Progress::Yielded
        }
    }
}

/// The state of the executor.
#[derive(Default)]
enum State {
    #[default]
    Idle,
    BurstTransfer(BurstTransfer),
    SliceTransfer(SliceTransfer),
    LinkedTransfer(LinkedTransfer),
}

fn update_master_interrupt(psx: &mut PSX) {
    if psx.dma.interrupt_control.update_master_interrupt_flag() {
        psx.interrupts.status.request(Interrupt::DMA);
    }
}

/// A DMA transfer executor.
#[derive(Default)]
pub struct Executor(State);

impl Executor {
    #[inline(always)]
    pub fn ongoing(&self) -> bool {
        !matches!(self.0, State::Idle)
    }

    pub fn advance(&mut self, psx: &mut PSX) {
        update_master_interrupt(psx);

        let (channel, progress) = match &mut self.0 {
            State::BurstTransfer(transfer) => (transfer.channel, transfer.advance(psx)),
            State::SliceTransfer(transfer) => (transfer.channel, transfer.advance(psx)),
            State::LinkedTransfer(transfer) => (transfer.channel, transfer.advance(psx)),
            State::Idle => unreachable!(),
        };

        match channel {
            Channel::GPU => {
                psx.scheduler.schedule(Event::Gpu, 0);
            }
            Channel::OTC => (),
            Channel::CDROM => {
                psx.scheduler
                    .schedule(Event::Cdrom(cdrom::Event::Update), 0);
            }
            _ => {
                error!(
                    psx.loggers.cdrom,
                    "advancing unimplemented channel: {channel:?}"
                )
            }
        }

        match progress {
            Progress::Ongoing => {
                psx.scheduler
                    .schedule(Event::DmaAdvance, channel.cycles_per_word());
            }
            Progress::Yielded => {
                trace!(
                    psx.loggers.dma,
                    "transfer on channel {channel:?} has yielded";
                );

                if psx
                    .dma
                    .interrupt_control
                    .channel_interrupt_mode_at(channel as usize)
                    .unwrap()
                    == crate::dma::ChannelInterruptMode::OnBlock
                {
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

                psx.scheduler
                    .schedule(Event::DmaAdvance, channel.cycles_per_word());
            }
            Progress::Finished => {
                info!(
                    psx.loggers.dma,
                    "finished transfer on channel {channel:?}";
                );

                self.0 = State::Idle;

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

        if matches!(self.0, State::Idle) {
            let mut enabled_channels = psx.dma.control.enabled_channels();
            enabled_channels.sort_unstable_by_key(|(_, priority)| std::cmp::Reverse(*priority));

            for (channel, _) in enabled_channels {
                let channel_state = &mut psx.dma.channels[channel as usize];
                if channel_state.control.transfer_ongoing() {
                    let dreq = match channel {
                        Channel::OTC => false,
                        Channel::GPU => {
                            psx.gpu.status.update_dreq();
                            psx.gpu.status.dma_request()
                        }
                        _ => true,
                    };

                    if !dreq && !channel_state.control.force_transfer() {
                        warn!(
                            psx.loggers.dma,
                            "{:?} DREQ not set: {:?}",
                            channel,
                            channel_state.control.clone()
                        );

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

                            self.0 = State::BurstTransfer(BurstTransfer {
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

                            self.0 = State::SliceTransfer(SliceTransfer { channel });
                        }
                        TransferMode::LinkedList => {
                            info!(
                                psx.loggers.dma,
                                "starting linked transfer on channel {channel:?}";
                            );

                            self.0 = State::LinkedTransfer(LinkedTransfer { channel });
                        }
                    }

                    psx.scheduler
                        .schedule(Event::DmaAdvance, channel.cycles_per_word());
                    return;
                }
            }
        }
    }
}
