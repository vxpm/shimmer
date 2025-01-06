use arrayvec::ArrayVec;
use bitos::prelude::*;
use integer::{u3, u7, u24};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaChannel {
    MdecIn = 0,
    MdecOut = 1,
    GPU = 2,
    CDROM = 3,
    SPU = 4,
    PIO = 5,
    OTC = 6,
}

#[bitos(1)]
#[derive(Debug, PartialEq, Eq)]
pub enum TransferDirection {
    DeviceToRam = 0x0,
    RamToDevice = 0x1,
}

#[bitos(1)]
#[derive(Debug, PartialEq, Eq)]
pub enum DataDirection {
    /// The address of the data to be transferred increases.
    Forward = 0x0,
    /// The address of the data to be transferred decreases.
    Backward = 0x1,
}

#[bitos(2)]
#[derive(Debug, PartialEq, Eq)]
pub enum TransferMode {
    /// Data is transferred all at once.
    Burst = 0x0,
    /// Data is transferred block by block.
    Slice = 0x1,
    LinkedList = 0x2,
}

/// Contains the base memory address where the DMA of channel `N` will start writing to/reading from.
#[bitos(32)]
#[derive(Debug)]
pub struct ChannelBase {
    #[bits(0..24)]
    addr: u24,
}

/// Used for configuring the blocks transferred in the DMA of channel `N`.
#[allow(clippy::len_without_is_empty)]
#[bitos(32)]
#[derive(Debug)]
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
#[derive(Debug)]
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

#[bitos(4)]
#[derive(Debug)]
pub struct ChannelStatus {
    /// The priority of this channel.
    #[bits(0..3)]
    pub priority: u3,
    /// Whether this channel is enabled or not.
    #[bits(3..4)]
    pub enabled: bool,
}

#[bitos(32)]
#[derive(Debug, Default)]
pub struct Control {
    /// The status of each channel.
    #[bits(0..28)]
    channel_status: [ChannelStatus; 7],
    /// The priority of the CPU for memory accesses.
    #[bits(28..31)]
    cpu_priority: u3,
}

impl Control {
    #[inline]
    pub fn enabled_channels(self) -> ArrayVec<(DmaChannel, u3), 7> {
        self.channel_status()
            .into_iter()
            .enumerate()
            .filter_map(|(i, channel)| {
                channel
                    .enabled()
                    .then_some(unsafe { (std::mem::transmute(i as u8), channel.priority()) })
            })
            .collect()
    }
}

#[bitos(1)]
#[derive(Debug, PartialEq, Eq)]
pub enum ChannelInterruptMode {
    /// The interrupt occurs only when the entire transfer completes.
    OnCompletion = 0x0,
    /// The interrupt occurs for every slice and linked-list transfer.
    OnSegment = 0x1,
}

/// Register that controls how DMA channels raise interrupts and which channels are actually
/// allowed to raise one.
#[bitos(32)]
#[derive(Debug, PartialEq, Eq, Default)]
pub struct DmaInterruptControl {
    /// The channel interrupt mode of each channel.
    #[bits(0..7)]
    channel_interrupt_mode: [ChannelInterruptMode; 7],
    /// A flag that gets raised when transferring to/from an address outside of RAM.
    #[bits(15..16)]
    bus_error: bool,
    /// Specifies which channels are allowed to raise an interrupt.
    #[bits(16..23)]
    channel_interrupt_mask: [bool; 7],
    /// Raw, numerical value of the `channel_interrupt_mask`.
    #[bits(16..23)]
    channel_interrupt_mask_raw: u7,
    /// Specifies if channels are allowed to raise an interrupt.
    #[bits(23..24)]
    master_channel_interrupt_enable: bool,
    /// Set whenever the transfer of the given channel completes (according to the mode in
    /// `channel_interrupt_mode`), but only if enabled in the channel interrupt mask. Writing 1 to
    /// these bits clears them.
    #[bits(24..31)]
    channel_interrupt_flags: [bool; 7],
    /// Raw, numerical value of the `channel_interupt_flags`.
    #[bits(24..31)]
    channel_interrupt_flags_raw: u7,
    /// A flag that gets raised when any channel that is allowed to raise an interrupt has a
    /// pending interrupt. Note that this takes into account the master interrupt enable and is
    /// forced if the bus error flag is set.
    #[bits(31..32)]
    master_interrupt_flag: bool,
}

impl DmaInterruptControl {
    /// If a DMA channel interrupt is being requested (i.e. a DMA interrupt should be
    /// triggered if master channel interrupt is enabled), returns the channel requesting it.
    #[inline]
    pub fn requested(self) -> Option<u8> {
        let requested =
            self.channel_interrupt_flags_raw().value() & self.channel_interrupt_mask_raw().value();
        let trailing = requested.trailing_zeros();
        (trailing != 8).then_some(trailing as u8)
    }

    /// Marks the transfer of a channel as complete according to it's [`ChannelInterruptMode`].
    #[inline]
    pub fn with_completed(self, channel: u8) -> Self {
        self.with_channel_interrupt_flags_at(channel as usize, true)
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
    pub interrupt_control: DmaInterruptControl,
}

impl State {
    // fn check_transfers(&mut self, bus: &mut Bus) {
    //     let dma_control = ctx.memory.read_reg::<DmaControl>();
    //
    //     let mut enabled_channels = dma_control.enabled_channels();
    //     enabled_channels.sort_unstable_by_key(|(_, priority)| std::cmp::Reverse(*priority));
    //
    //     for (channel, _) in enabled_channels {
    //         let channel_control = read_dma_channel_control(ctx.memory, channel);
    //         if channel_control.transfer_ongoing() {
    //             println!("{channel:?} ongoing")
    //         }
    //     }
    // }
    //
    // pub fn cycle(&mut self, ctx: &mut Ctx) {
    //     self.check_transfers(ctx);
    // }
}
