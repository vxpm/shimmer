pub mod executor;

use arrayvec::ArrayVec;
use bitos::prelude::*;
use integer::{u3, u7, u24};

pub use executor::Executor;

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
    #[bits(0)]
    pub transfer_direction: TransferDirection,
    /// Direction of the data to be transferred.
    #[bits(1)]
    pub data_direction: DataDirection,
    /// When enabled, causes alternative behaviour depending on the transfer mode:
    /// - Burst: Enables CPU cycle stealing
    /// - Slice: Causes DMA to hang
    /// - Linked-List: ?
    #[bits(8)]
    pub alternative_behaviour: bool,
    /// The mode of operation for the transfer.
    #[bits(9..11)]
    pub transfer_mode: Option<TransferMode>,
    #[bits(16..19)]
    pub chopping_dma_window_size: u3,
    #[bits(20..23)]
    pub chopping_cpu_window_size: u3,
    /// Whether a transfer is in progress or not.
    #[bits(24)]
    pub transfer_ongoing: bool,
    /// Forces the transfer to start without waiting for the DREQ.
    #[bits(28)]
    pub force_transfer: bool, // NOTE: DREQ refers to the hardware signal
    #[bits(30)]
    pub bus_snooping: bool,
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
    /// Updates the master interrupt flag and returns whether it performed a low-to-high
    /// transition.
    pub fn update_master_interrupt_flag(&mut self) -> bool {
        let old = self.master_interrupt_flag();
        self.set_master_interrupt_flag(
            self.bus_error()
                || (self.master_channel_interrupt_enable()
                    && self.channel_interrupt_flags_raw().value() != 0),
        );

        !old && self.master_interrupt_flag()
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

pub struct State {
    pub control: Control,
    pub interrupt_control: InterruptControl,

    pub channels: [ChannelState; 7],
}

impl Default for State {
    fn default() -> Self {
        let mut channels: [ChannelState; 7] = Default::default();
        channels[6]
            .control
            .set_data_direction(DataDirection::Backward);

        Self {
            control: Default::default(),
            interrupt_control: Default::default(),
            channels,
        }
    }
}
