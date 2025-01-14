//! Items related to the DMA (Direct Memory Access) controller of the PSX.

mod executor;

use arrayvec::ArrayVec;
use bitos::prelude::*;
use integer::{u3, u7, u24};

pub use executor::Executor;

/// A DMA channel.
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

impl Channel {
    /// How many cycles it takes to transfer a single word in this DMA channel.
    pub fn cycles_per_word(&self) -> u64 {
        match self {
            Channel::MdecIn => 1,
            Channel::MdecOut => 1,
            Channel::GPU => 1,
            Channel::CDROM => 24,
            Channel::SPU => 4,
            Channel::PIO => 20,
            Channel::OTC => 1,
        }
    }
}

/// The direction of a DMA transfer.
#[bitos(1)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDirection {
    /// Copying from some source in the device to a destination address in RAM (the channel base).
    DeviceToRam = 0x0,
    /// Copying from a source address in RAM (the channel base) to some destination in the device.
    RamToDevice = 0x1,
}

/// The direction of data in a DMA transfer.
#[bitos(1)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataDirection {
    /// The address of the data to be transferred increases.
    Forward = 0x0,
    /// The address of the data to be transferred decreases.
    Backward = 0x1,
}

/// Modes a transfer can be executed in.
#[bitos(2)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferMode {
    /// Data is transferred all at once.
    Burst = 0x0,
    /// Data is transferred block by block.
    Slice = 0x1,
    /// Data is transferred through a linked list of data nodes.
    LinkedList = 0x2,
}

/// Contains the base memory address of a DMA channel.
#[bitos(32)]
#[derive(Debug, Clone, Default)]
pub struct ChannelBase {
    #[bits(0..24)]
    pub addr: u24,
}

/// Configuration of the blocks transferred through a DMA channel.
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

/// Configuration of a DMA channel.
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

/// The state of a DMA channel.
#[derive(Debug, Clone, Default)]
pub struct ChannelState {
    pub base: ChannelBase,
    pub block_control: ChannelBlockControl,
    pub control: ChannelControl,
}

/// Configuration of a channel in the DMA controller's configuration.
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

/// Configuration of the DMA controller regarding channels.
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

/// How interrupts should be raised for a given channel.
#[bitos(1)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelInterruptMode {
    /// The interrupt occurs only when the entire transfer completes.
    OnCompletion = 0x0,
    /// The interrupt occurs for every slice and linked-list transfer.
    OnBlock = 0x1,
}

/// Configuration of the DMA controller regarding interrupts.
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

/// The state of the DMA controller.
pub struct Controller {
    pub control: Control,
    pub interrupt_control: InterruptControl,
    pub channels: [ChannelState; 7],
}

impl Default for Controller {
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
