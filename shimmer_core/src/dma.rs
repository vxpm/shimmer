use crate::{PSX, cpu::cop0::Interrupt, gpu, mem::Address};
use arrayvec::ArrayVec;
use bitos::prelude::*;
use integer::{u3, u7, u24};
use tinylog::{debug, info};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Channel {
    MdecIn = 0,
    MdecOut = 1,
    GPU = 2,
    CDROM = 3,
    SPU = 4,
    PIO = 5,
    OTC = 6,
}

#[bitos(1)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDirection {
    DeviceToRam = 0x0,
    RamToDevice = 0x1,
}

#[bitos(1)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataDirection {
    /// The address of the data to be transferred increases.
    Forward = 0x0,
    /// The address of the data to be transferred decreases.
    Backward = 0x1,
}

#[bitos(2)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferMode {
    /// Data is transferred all at once.
    Burst = 0x0,
    /// Data is transferred block by block.
    Slice = 0x1,
    LinkedList = 0x2,
}

/// Contains the base memory address where the DMA of channel `N` will start writing to/reading from.
#[bitos(32)]
#[derive(Debug, Clone, Default)]
pub struct ChannelBase {
    #[bits(0..24)]
    addr: u24,
}

/// Used for configuring the blocks transferred in the DMA of channel `N`.
#[allow(clippy::len_without_is_empty)]
#[bitos(32)]
#[derive(Debug, Clone, Default)]
pub struct ChannelBlockControl {
    /// The size of a single block in words.
    #[bits(0..16)]
    pub len: u16,
    /// The amount of blocks to transfer.
    #[bits(16..32)]
    pub count: u16,
}

/// Used for configuring the blocks transferred in the DMA of channel `N`.
#[bitos(32)]
#[derive(Debug, Clone, Default)]
pub struct ChannelControl {
    /// Direction of the DMA transfer.
    #[bits(0..1)]
    pub transfer_direction: TransferDirection,
    /// Direction of the data to be transferred.
    #[bits(1..2)]
    pub data_direction: DataDirection,
    /// When enabled, causes alternative behaviour depending on the transfer mode:
    /// - Burst: Enables CPU cycle stealing
    /// - Slice: Causes DMA to hang
    /// - Linked-List: ?
    #[bits(8..9)]
    pub alternative_behaviour: bool,
    /// The mode of operation for the transfer.
    #[bits(9..11)]
    pub transfer_mode: Option<TransferMode>,
    #[bits(16..19)]
    pub chopping_dma_window_size: u3,
    #[bits(20..23)]
    pub chopping_cpu_window_size: u3,
    /// Whether a transfer is in progress or not.
    #[bits(24..25)]
    pub transfer_ongoing: bool,
    /// Forces the transfer to start without waiting for the DREQ.
    #[bits(28..29)]
    pub force_transfer: bool, // NOTE: DREQ refers to the hardware signal
}

#[derive(Debug, Clone, Default)]
pub struct ChannelState {
    pub base: ChannelBase,
    pub block_control: ChannelBlockControl,
    pub control: ChannelControl,
}

#[bitos(4)]
#[derive(Debug, Clone)]
pub struct ChannelStatus {
    /// The priority of this channel.
    #[bits(0..3)]
    pub priority: u3,
    /// Whether this channel is enabled or not.
    #[bits(3..4)]
    pub enabled: bool,
}

#[bitos(32)]
#[derive(Debug, Clone, Default)]
pub struct Control {
    /// The status of each channel.
    #[bits(0..28)]
    pub channel_status: [ChannelStatus; 7],
    /// The priority of the CPU for memory accesses.
    #[bits(28..31)]
    pub cpu_priority: u3,
}

impl Control {
    #[inline]
    pub fn enabled_channels(&self) -> ArrayVec<(Channel, u3), 7> {
        let mut result = ArrayVec::new_const();
        let iter = self
            .channel_status()
            .into_iter()
            .enumerate()
            .filter_map(|(i, channel)| {
                channel.enabled().then_some(unsafe {
                    (
                        std::mem::transmute::<u8, Channel>(i as u8),
                        channel.priority(),
                    )
                })
            });

        for channel in iter {
            unsafe { result.push_unchecked(channel) };
        }

        result
    }
}

#[bitos(1)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelInterruptMode {
    /// The interrupt occurs only when the entire transfer completes.
    OnCompletion = 0x0,
    /// The interrupt occurs for every slice and linked-list transfer.
    OnSegment = 0x1,
}

/// Register that controls how DMA channels raise interrupts and which channels are actually
/// allowed to raise one.
#[bitos(32)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InterruptControl {
    /// The channel interrupt mode of each channel.
    #[bits(0..7)]
    pub channel_interrupt_mode: [ChannelInterruptMode; 7],
    /// A flag that gets raised when transferring to/from an address outside of RAM.
    #[bits(15..16)]
    pub bus_error: bool,
    /// Specifies which channels are allowed to raise an interrupt.
    #[bits(16..23)]
    pub channel_interrupt_mask: [bool; 7],
    /// Raw, numerical value of the `channel_interrupt_mask`.
    #[bits(16..23)]
    pub channel_interrupt_mask_raw: u7,
    /// Specifies if channels are allowed to raise an interrupt.
    #[bits(23..24)]
    pub master_channel_interrupt_enable: bool,
    /// Set whenever the transfer of the given channel completes (according to the mode in
    /// `channel_interrupt_mode`), but only if enabled in the channel interrupt mask. Writing 1 to
    /// these bits clears them.
    #[bits(24..31)]
    pub channel_interrupt_flags: [bool; 7],
    /// Raw, numerical value of the `channel_interupt_flags`.
    #[bits(24..31)]
    pub channel_interrupt_flags_raw: u7,
    /// A flag that gets raised when any channel that is allowed to raise an interrupt has a
    /// pending interrupt. Note that this takes into account the master interrupt enable and is
    /// forced if the bus error flag is set.
    #[bits(31..32)]
    pub master_interrupt_flag: bool,
}

impl InterruptControl {
    /// If a DMA channel interrupt is being requested (i.e. a DMA interrupt should be
    /// triggered if master channel interrupt is enabled), returns the channel requesting it.
    #[inline]
    pub fn requested(self) -> Option<u8> {
        let requested =
            self.channel_interrupt_flags_raw().value() & self.channel_interrupt_mask_raw().value();
        let trailing = requested.trailing_zeros();
        (trailing != 8).then_some(trailing as u8)
    }
}

// DMA Controller Behaviour
//
// # Summary
// The DMA controller is responsible for actually executing the DMAs and for raising interrupts
// when needed. It has two main registers:
// - DMA Control (DMAC): Controls which DMA channels are enabled and their priorities. When a DMA
// needs to be chosen, the one with highest priority wins.
// - DMA Interrupt Control (DMAIC): Controls how interrupts are raised for each channel, whether
// they are allowed to raise an interrupt and whether they want to raise one.
//
// It also has a bunch of registers for each of the DMA channels:
// - DMA Channel Control N (DMACCN): Controls the behaviour of DMA channel N.
// - DMA Block Control N (DMABCN): Controls the size and the amount of transfer blocks for DMA
// channel N.
// - DMA Base N: Contains the base address for the DMA transfer.
//
// # Transfer
// A transfer in channel N starts when:
// - DMACCN has `transfer_ongoing` set by the CPU.
// - The channel is enabled in DMAC and it has the highest priority of all enabled channels.
//
// When a transfer starts, the DMA controller transfers data as specified by the channel control.
// Transfers have 3 modes:
// - Burst: Transfer all at once.
// - Slice: Split data into blocks and transfer a single block per DMA request.
// - Linked-List: Transfer happens in blocks, like in Slice, but the blocks are not contiguous.
//
// Note that multiple transfers can happen at the same time:
//
// When the transfer in channel N finishes, DMACCN `transfer_ongoing` must be unset.
//
// # Interrupts
// Whenever a channel has its interrupt flag raised, the DMA controller must trigger the DMA
// interrupt (according to the DMA interrupt mode of the channel that completed) if the channel
// interrupt is enabled in the mask.
//
// 01. Check channels that have completed (interrupt flag = 1)
// 02. Mask channels allowed to raise interrupts (interrupt mask = 1)
// 03. Raise interrupt!

#[derive(Default)]
pub struct State {
    pub control: Control,
    pub interrupt_control: InterruptControl,

    pub channels: [ChannelState; 7],
}

fn transfer_burst(psx: &mut PSX, channel: Channel) {
    let channel_base = &psx.dma.channels[channel as usize].base;
    let channel_block_control = &psx.dma.channels[channel as usize].block_control;

    match channel {
        Channel::OTC => {
            let base = channel_base.addr().value() & !0b11;
            let entries = if channel_block_control.len() == 0 {
                0x10000
            } else {
                u32::from(channel_block_control.len())
            };

            debug!(
                psx.loggers.dma,
                "base = {}, entries = {entries}",
                Address(base)
            );

            let mut addr = base;
            for _ in 1..entries {
                let prev = addr.wrapping_sub(4) & 0x00FF_FFFF;
                psx.write::<_, true>(Address(addr), prev).unwrap();

                let region = Address(addr).physical().and_then(|p| p.region());
                debug!(
                    psx.loggers.dma,
                    "[{}] = {} ({region:?})",
                    Address(addr),
                    Address(prev)
                );

                addr = prev;
            }

            psx.write::<_, true>(Address(addr), 0x00FF_FFFF).unwrap();
            debug!(psx.loggers.dma, "[{}] = 0x00FF_FFFF", Address(addr));
            debug!(
                psx.loggers.dma,
                "FINISHED - addr: {}",
                psx.cpu.instr_delay_slot().1
            );
        }
        _ => todo!(),
    }
}

fn transfer_slice(psx: &mut PSX, channel: Channel) {
    match channel {
        Channel::OTC => transfer_burst(psx, channel),
        _ => todo!(),
    }
}

fn transfer_linked(psx: &mut PSX, channel: Channel) {
    let channel_base = &psx.dma.channels[channel as usize].base;

    match channel {
        Channel::OTC => transfer_burst(psx, channel),
        Channel::GPU => {
            let mut current = channel_base.addr().value() & !0b11;
            loop {
                let node = psx.read::<u32, true>(Address(current)).unwrap();
                let next = node.bits(0, 24);
                let words = node.bits(24, 32);

                if next == 0x00FF_FFFF {
                    break;
                }

                for i in 0..words {
                    let addr = current + (i + 1) * 4;
                    let word = psx.read::<u32, true>(Address(addr)).unwrap();
                    psx.gpu.queue.push_back(gpu::instr::Instruction::Rendering(
                        gpu::instr::RenderingInstruction::from_bits(word),
                    ));
                }

                current = next & !0b11;
            }
        }
        _ => todo!(),
    }
}

pub fn check_transfers(psx: &mut PSX) {
    let mut enabled_channels = psx.dma.control.enabled_channels();
    enabled_channels.sort_unstable_by_key(|(_, priority)| std::cmp::Reverse(*priority));

    for (channel, _) in enabled_channels {
        let channel_control = &psx.dma.channels[channel as usize].control;
        if channel_control.transfer_ongoing() {
            info!(psx.loggers.dma, "{channel:?} ongoing"; control = channel_control.clone());

            match channel_control
                .transfer_mode()
                .unwrap_or(TransferMode::Burst)
            {
                TransferMode::Burst => transfer_burst(psx, channel),
                TransferMode::Slice => transfer_slice(psx, channel),
                TransferMode::LinkedList => transfer_linked(psx, channel),
            }

            let channel_control = &mut psx.dma.channels[channel as usize].control;
            channel_control.set_transfer_ongoing(false);
            channel_control.set_force_transfer(false);

            let interrupt_control = &mut psx.dma.interrupt_control;
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
                psx.cop0.interrupt_status.request(Interrupt::DMA);
            }
        }
    }
}
