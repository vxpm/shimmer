//! Items related to the memory of the PSX.

pub mod io;

mod primitive;

use crate::{PSX, cdrom, dma, exe::Executable, scheduler::Event, util};
use binrw::BinRead;
use bitos::integer::u7;
use easyerr::Error;
use tinylog::{debug, trace, warn};
use zerocopy::IntoBytes;

pub use primitive::{Primitive, PrimitiveRw};

/// A memory segment refers to a specific range of memory addresses, each with it's own purpose and
/// properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Segment {
    /// Kernel User SEGment
    ///
    /// Intended to be used as user virtual memory. In user mode, this is the only accessible
    /// segment. The PSX, however, does not have a MMU, and so does not use it for virtual memory.
    /// It is instead a simple mirror of the KSEG0/KSEG1 in the first 512MiB.
    KUSEG,
    /// Kernel SEGment 0
    ///
    /// Maps to the physical memory directly, utilizing the cache. Only accessible in kernel mode.
    KSEG0,
    /// Kernel SEGment 1
    ///
    /// Maps to the physical memory directly and does not utilize the cache. Only accessible in
    /// kernel mode.
    KSEG1,
    /// Kernel SEGment 2
    ///
    /// Intended to be used as kernel virtual memory. The PSX, however, does not have a MMU, and so
    /// uses it for internal, memory mapped CPU control registers.
    KSEG2,
}

impl Segment {
    #[inline(always)]
    pub const fn start(&self) -> Address {
        match self {
            Segment::KUSEG => Address(0x0000_0000),
            Segment::KSEG0 => Address(0x8000_0000),
            Segment::KSEG1 => Address(0xA000_0000),
            Segment::KSEG2 => Address(0xC000_0000),
        }
    }
}

/// A memory region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    Ram,
    RamMirror,
    Expansion1,
    ScratchPad,
    IOPorts,
    Expansion2,
    Expansion3,
    BIOS,
}

#[expect(clippy::len_without_is_empty, reason = "not a collection")]
impl Region {
    /// The length of this region, in bytes.
    #[inline(always)]
    pub const fn start(&self) -> PhysicalAddress {
        // SAFETY: the addresses are in the physical range
        unsafe {
            PhysicalAddress::new_unchecked(match self {
                Region::Ram => 0x0000_0000,
                Region::RamMirror => 0x0020_0000,
                Region::Expansion1 => 0x1F00_0000,
                Region::ScratchPad => 0x1F80_0000,
                Region::IOPorts => 0x1F80_1000,
                Region::Expansion2 => 0x1F80_2000,
                Region::Expansion3 => 0x1FA0_0000,
                Region::BIOS => 0x1FC0_0000,
            })
        }
    }

    /// The length of this region, in bytes.
    #[inline(always)]
    pub const fn len(&self) -> u32 {
        match self {
            Region::Ram => 2 * bytesize::MIB as u32,
            Region::RamMirror => 6 * bytesize::MIB as u32,
            Region::Expansion1 => 8 * bytesize::MIB as u32,
            Region::ScratchPad => bytesize::KIB as u32,
            Region::IOPorts => 8 * bytesize::KIB as u32,
            Region::Expansion2 => 8 * bytesize::KIB as u32,
            Region::Expansion3 => 2 * bytesize::MIB as u32,
            Region::BIOS => 4 * bytesize::MIB as u32,
        }
    }
}

/// A physical memory address. This is a thin wrapper around a [`u32`], with the extra guarantee
/// that it's in the `0x0000_0000..0x2000_0000` range (512 MiB).
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd)]
pub struct PhysicalAddress(u32);

impl std::fmt::Display for PhysicalAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "0x{:04X}_{:04X}",
            (self.0 & 0xFFFF_0000) >> 16,
            self.0 & 0xFFFF
        )
    }
}

impl std::fmt::Debug for PhysicalAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl PhysicalAddress {
    /// Creates a new [`PhysicalAddress`] from an absolute address.
    #[inline(always)]
    pub const fn new(address: u32) -> Option<Self> {
        if address < 0x2000_0000 {
            Some(Self(address))
        } else {
            None
        }
    }

    /// Creates a new [`PhysicalAddress`] from an absolute address, without checking.
    ///
    /// # Safety
    /// `address` must be in the `0x0000_0000..0x2000_0000` range.
    #[inline(always)]
    pub const unsafe fn new_unchecked(address: u32) -> Self {
        debug_assert!(address < 0x2000_0000);
        Self(address)
    }

    #[inline(always)]
    pub const fn value(&self) -> u32 {
        let value = self.0;

        // SAFETY: this is an invariant of this type
        unsafe { std::hint::assert_unchecked(value < 0x2000_0000) };
        value
    }

    #[inline(always)]
    pub const fn region(&self) -> Option<Region> {
        macro_rules! check {
            ($($region:expr),*) => {
                match self.value() {
                    $(
                        const { $region.start().value() }
                        ..const { $region.start().value() + $region.len() }
                        => Some($region),
                    )*
                    _ => None,
                }
            };
        }

        check!(
            Region::Ram,
            Region::RamMirror,
            Region::Expansion1,
            Region::ScratchPad,
            Region::IOPorts,
            Region::Expansion2,
            Region::Expansion3,
            Region::BIOS
        )
    }
}

/// A virtual memory address. This is a thin wrapper around a [`u32`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, BinRead)]
pub struct Address(pub u32);

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "0x{:04X}_{:04X}",
            (self.0 & 0xFFFF_0000) >> 16,
            self.0 & 0xFFFF
        )
    }
}

impl std::fmt::Debug for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Address {
    /// Returns the value of this address. Equivalent to `self.0`.
    #[inline(always)]
    pub const fn value(self) -> u32 {
        self.0
    }

    /// Returns `true` if this address is aligned to the given alignment.
    #[inline(always)]
    pub const fn is_aligned(self, alignment: u32) -> bool {
        self.0 % alignment == 0
    }

    /// Returns the segment of this address.
    #[inline(always)]
    pub const fn segment(self) -> Segment {
        match self.0 {
            0x0000_0000..=0x7FFF_FFFF => Segment::KUSEG,
            0x8000_0000..=0x9FFF_FFFF => Segment::KSEG0,
            0xA000_0000..=0xBFFF_FFFF => Segment::KSEG1,
            0xC000_0000..=0xFFFF_FFFF => Segment::KSEG2,
        }
    }

    /// Returns the physical address that this virtual address maps to.
    ///
    /// If the [`segment`](Self::segment) of this address is `KUSEG | KSEG0 | KSEG1`, this is
    /// somewhere in `0000_0000..0x2000_0000`. Otherwise, it's in `KSEG2` and does not map to a
    /// physical address.
    #[inline(always)]
    pub const fn physical(self) -> Option<PhysicalAddress> {
        PhysicalAddress::new(match self.segment() {
            Segment::KSEG2 => self.0,
            _ => self.0 & 0x1FFF_FFFF,
        })
    }
}

impl std::ops::Add<u32> for Address {
    type Output = Self;

    fn add(self, rhs: u32) -> Self::Output {
        Self(self.0.wrapping_add(rhs))
    }
}

impl std::ops::Add<i32> for Address {
    type Output = Self;

    fn add(self, rhs: i32) -> Self::Output {
        Self(self.0.wrapping_add_signed(rhs))
    }
}

impl std::ops::Sub<u32> for Address {
    type Output = Self;

    fn sub(self, rhs: u32) -> Self::Output {
        Self(self.0.wrapping_sub(rhs))
    }
}

impl std::ops::Sub<i32> for Address {
    type Output = Self;

    fn sub(self, rhs: i32) -> Self::Output {
        Self(self.0.wrapping_add_signed(-rhs))
    }
}

impl PartialEq<u32> for Address {
    fn eq(&self, other: &u32) -> bool {
        self.0 == *other
    }
}

pub type BoxedU8Arr<const LEN: usize> = Box<[u8; LEN]>;

/// Collection of memory components, e.g. RAM, BIOS and the Scratchpad.
pub struct Memory {
    /// Main RAM (the first 2 MB).
    pub ram: BoxedU8Arr<{ Region::Ram.len() as usize }>,
    /// Expansion 1
    pub expansion_1: BoxedU8Arr<{ Region::Expansion1.len() as usize }>,
    /// Scratchpad or Fast RAM.
    pub scratchpad: BoxedU8Arr<{ Region::ScratchPad.len() as usize }>,
    // expansion region 2
    pub expansion_2: BoxedU8Arr<{ Region::Expansion2.len() as usize }>,
    // expansion region 3
    pub expansion_3: BoxedU8Arr<{ Region::Expansion3.len() as usize }>,
    /// BIOS ROM.
    pub bios: BoxedU8Arr<{ Region::BIOS.len() as usize }>,
    /// Some IO Ports are stubbed to write and read from this buffer.
    pub io_stubs: BoxedU8Arr<{ Region::IOPorts.len() as usize }>,
    /// Executable to side load, if any.
    pub sideload: Option<Executable>,
    /// Kernel STDOUT.
    pub kernel_stdout: String,
}

impl Memory {
    /// Creates a new [`Memory`] with zeroed contents and the given BIOS ROM.
    ///
    /// # Errors
    /// If the bios is larger than 4096 KB, it's too big to fit and so [`Err`]
    /// is returned with the given bios.
    pub fn with_bios(mut bios: Vec<u8>) -> Result<Self, Vec<u8>> {
        if bios.len() > Region::BIOS.len() as usize {
            return Err(bios);
        }

        bios.resize(Region::BIOS.len() as usize, 0);
        Ok(Self {
            ram: util::boxed_array(0),
            expansion_1: util::boxed_array(0),
            expansion_2: util::boxed_array(0),
            expansion_3: util::boxed_array(0),
            scratchpad: util::boxed_array(0),
            bios: Box::try_from(bios.into_boxed_slice())
                .expect("boxed slice of the bios data should be exactly 4096 KiB big"),
            io_stubs: util::boxed_array(0),

            sideload: None,
            kernel_stdout: String::new(),
        })
    }
}

#[derive(Debug, Clone, Copy, Error)]
#[error("address {addr} is misaligned (expected alignment of {alignment})")]
pub struct MisalignedAddressErr {
    pub addr: Address,
    pub alignment: u32,
}

/// Helper function to perform masked writes.
fn write_masked<P: Primitive, B: bitos::Bits>(src: P, offset: usize, mask: B::Bits, dst: &mut B)
where
    B::Bits: zerocopy::IntoBytes
        + zerocopy::FromBytes
        + std::ops::BitAnd<B::Bits, Output = B::Bits>
        + std::ops::BitOr<B::Bits, Output = B::Bits>
        + std::ops::Not<Output = B::Bits>,
{
    let current = dst.to_bits();

    let mut buf = current;
    let bytes = buf.as_mut_bytes();
    src.write_to(&mut bytes[offset..]);

    let new = (current & !mask) | (buf & mask);
    *dst = B::from_bits(new);
}

impl PSX {
    fn read_io_ports<P, const SILENT: bool>(&mut self, addr: Address) -> P
    where
        P: Primitive,
    {
        let default = || {
            let offset = addr.physical().unwrap().value() - Region::IOPorts.start().value();
            P::read_from_buf(&self.memory.io_stubs[offset as usize..])
        };

        if let Some((reg, offset)) = io::Reg::reg_and_offset(addr) {
            if !SILENT {
                let ignore_list = [io::Reg::SramFifo, io::Reg::SpuControl, io::Reg::SpuStatus];
                if !ignore_list.contains(&reg) && !reg.is_spu_voice() {
                    trace!(
                        self.loggers.bus,
                        "{} bytes read from {reg:?}[{}..{}] ({})",
                        size_of::<P>(),
                        offset,
                        offset + size_of::<P>(),
                        addr,
                    );
                }
            }

            let read = match reg {
                io::Reg::InterruptStatus => {
                    let bytes = self.interrupts.status.as_bytes();
                    P::read_from_buf(&bytes[offset..])
                }
                io::Reg::InterruptMask => {
                    let bytes = self.interrupts.mask.as_bytes();
                    P::read_from_buf(&bytes[offset..])
                }
                io::Reg::Dma0Base
                | io::Reg::Dma1Base
                | io::Reg::Dma2Base
                | io::Reg::Dma3Base
                | io::Reg::Dma4Base
                | io::Reg::Dma5Base
                | io::Reg::Dma6Base => {
                    let channel = reg.dma_channel().unwrap();
                    let bytes = self.dma.channels[channel as usize].base.as_bytes();
                    P::read_from_buf(&bytes[offset..])
                }
                io::Reg::Dma0BlockControl
                | io::Reg::Dma1BlockControl
                | io::Reg::Dma2BlockControl
                | io::Reg::Dma3BlockControl
                | io::Reg::Dma4BlockControl
                | io::Reg::Dma5BlockControl
                | io::Reg::Dma6BlockControl => {
                    let channel = reg.dma_channel().unwrap();
                    let bytes = self.dma.channels[channel as usize].block_control.as_bytes();
                    P::read_from_buf(&bytes[offset..])
                }
                io::Reg::Dma0Control
                | io::Reg::Dma1Control
                | io::Reg::Dma2Control
                | io::Reg::Dma3Control
                | io::Reg::Dma4Control
                | io::Reg::Dma5Control
                | io::Reg::Dma6Control => {
                    let channel = reg.dma_channel().unwrap();
                    let bytes = self.dma.channels[channel as usize].control.as_bytes();
                    P::read_from_buf(&bytes[offset..])
                }
                io::Reg::DmaControl => {
                    let bytes = self.dma.control.as_bytes();
                    P::read_from_buf(&bytes[offset..])
                }
                io::Reg::DmaInterrupt => {
                    let bytes = self.dma.interrupt_control.as_bytes();
                    P::read_from_buf(&bytes[offset..])
                }
                io::Reg::Gp0 => {
                    let value = self.gpu.response_queue.pop_front();
                    let value = if let Some(value) = value {
                        value
                    } else {
                        warn!(self.loggers.gpu, "reading from empty response queue");
                        0
                    };

                    P::read_from_buf(&value.as_bytes()[offset..])
                }
                io::Reg::Gp1 => {
                    let bytes = self.gpu.status.as_bytes();
                    trace!(self.loggers.bus, "{:?}", self.gpu.status);

                    P::read_from_buf(&bytes[offset..])
                }
                io::Reg::Cdrom0 | io::Reg::Cdrom1 | io::Reg::Cdrom2 | io::Reg::Cdrom3 => {
                    let reg = reg.cdrom_reg().unwrap();
                    self.scheduler
                        .schedule(Event::Cdrom(cdrom::Event::Update), 0);
                    P::read_from_buf(self.cdrom.read(reg).as_bytes())
                }
                io::Reg::Timer2Value => {
                    let bytes = self.timers.timer2.value.as_bytes();
                    P::read_from_buf(&bytes[offset..])
                }
                io::Reg::Timer2Mode => {
                    let value = self.timers.timer2.mode.to_bits();
                    let bytes = value.as_bytes();

                    P::read_from_buf(&bytes[offset..])
                }
                io::Reg::Timer2Target => {
                    let bytes = self.timers.timer2.target.as_bytes();
                    P::read_from_buf(&bytes[offset..])
                }
                io::Reg::JoyStat => P::read_from_buf(&[0xFF, 0xFF, 0xFF, 0xFF]),
                _ => default(),
            };

            read
        } else {
            if !SILENT {
                warn!(
                    self.loggers.bus,
                    "{} bytes read from unknown IO port {}",
                    size_of::<P>(),
                    addr,
                );
            }

            default()
        }
    }

    pub fn read_unaligned<P, const SILENT: bool>(&mut self, addr: Address) -> P
    where
        P: Primitive,
    {
        if let Some(phys) = addr.physical() {
            let Some(region) = phys.region() else {
                if !SILENT {
                    warn!(
                        self.loggers.bus,
                        "read from {addr} ({phys}) which is in an unknown region"
                    );
                }

                return [0, 0, 0, 0].read();
            };

            let offset = phys.value() - region.start().value();
            match region {
                Region::Ram => self.memory.ram[offset as usize..].read(),
                Region::RamMirror => self.memory.ram[(offset & 0x001F_FFFF) as usize..].read(),
                Region::Expansion1 => self.memory.expansion_1[offset as usize..].read(),
                Region::ScratchPad => self.memory.scratchpad[offset as usize..].read(),
                Region::IOPorts => self.read_io_ports::<P, SILENT>(addr),
                Region::Expansion2 => self.memory.expansion_2[offset as usize..].read(),
                Region::Expansion3 => self.memory.expansion_3[offset as usize..].read(),
                Region::BIOS => self.memory.bios[offset as usize..].read(),
            }
        } else {
            self.cpu.cache_control().as_bytes().read()
        }
    }

    #[inline(always)]
    pub fn read<P, const SILENT: bool>(&mut self, addr: Address) -> Result<P, MisalignedAddressErr>
    where
        P: Primitive,
    {
        (addr.is_aligned(P::ALIGNMENT))
            .then(|| self.read_unaligned::<P, SILENT>(addr))
            .ok_or(MisalignedAddressErr {
                addr,
                alignment: P::ALIGNMENT,
            })
    }

    fn write_io_ports<P, const SILENT: bool>(&mut self, addr: Address, value: P)
    where
        P: Primitive,
    {
        let mut default = || {
            let offset = addr.physical().unwrap().value() - Region::IOPorts.start().value();
            value.write_to(&mut self.memory.io_stubs[offset as usize..])
        };

        if let Some((reg, offset)) = io::Reg::reg_and_offset(addr) {
            if !SILENT {
                let ignore_list = [io::Reg::SramFifo, io::Reg::SpuControl, io::Reg::SpuStatus];
                if !ignore_list.contains(&reg) && !reg.is_spu_voice() {
                    debug!(
                        self.loggers.bus,
                        "{} bytes written to {reg:?}[{}..{}] ({}): 0x{:X?}",
                        size_of::<P>(),
                        offset,
                        offset + size_of::<P>(),
                        addr,
                        value,
                    );
                }
            }

            match reg {
                io::Reg::InterruptStatus => {
                    let stat_bytes = &mut self.interrupts.status.as_mut_bytes()[offset..];
                    let value_bytes = value.as_bytes();

                    for (value_byte, stat_byte) in value_bytes.iter().zip(stat_bytes) {
                        *stat_byte &= value_byte;
                    }
                }
                io::Reg::InterruptMask => {
                    let reg_bytes = self.interrupts.mask.as_mut_bytes();
                    value.write_to(&mut reg_bytes[offset..]);
                }
                io::Reg::Dma0Base
                | io::Reg::Dma1Base
                | io::Reg::Dma2Base
                | io::Reg::Dma3Base
                | io::Reg::Dma4Base
                | io::Reg::Dma5Base
                | io::Reg::Dma6Base => {
                    let channel = reg.dma_channel().unwrap();
                    let bytes = self.dma.channels[channel as usize].base.as_mut_bytes();
                    value.write_to(&mut bytes[offset..]);

                    self.scheduler.schedule(Event::DmaUpdate, 0);
                }
                io::Reg::Dma0BlockControl
                | io::Reg::Dma1BlockControl
                | io::Reg::Dma2BlockControl
                | io::Reg::Dma3BlockControl
                | io::Reg::Dma4BlockControl
                | io::Reg::Dma5BlockControl
                | io::Reg::Dma6BlockControl => {
                    let channel = reg.dma_channel().unwrap();
                    let bytes = self.dma.channels[channel as usize]
                        .block_control
                        .as_mut_bytes();
                    value.write_to(&mut bytes[offset..]);

                    self.scheduler.schedule(Event::DmaUpdate, 0);
                }
                io::Reg::Dma0Control
                | io::Reg::Dma1Control
                | io::Reg::Dma2Control
                | io::Reg::Dma3Control
                | io::Reg::Dma4Control
                | io::Reg::Dma5Control => {
                    let channel = reg.dma_channel().unwrap();
                    let bytes = self.dma.channels[channel as usize].control.as_mut_bytes();
                    value.write_to(&mut bytes[offset..]);

                    self.scheduler.schedule(Event::DmaUpdate, 0);
                }
                io::Reg::Dma6Control => {
                    write_masked(
                        value,
                        offset,
                        dma::ChannelControl::DMA6_WRITE_MASK as u32,
                        &mut self.dma.channels[6].control,
                    );

                    self.scheduler.schedule(Event::DmaUpdate, 0);
                }
                io::Reg::DmaControl => {
                    let bytes = self.dma.control.as_mut_bytes();
                    value.write_to(&mut bytes[offset..]);

                    self.scheduler.schedule(Event::DmaUpdate, 0);
                }
                io::Reg::DmaInterrupt => {
                    let mut result = self.dma.interrupt_control.clone();
                    write_masked(
                        value,
                        offset,
                        dma::InterruptControl::WRITE_MASK as u32,
                        &mut result,
                    );

                    // reset interrupt flags
                    let reset = self
                        .dma
                        .interrupt_control
                        .channel_interrupt_flags_raw()
                        .value()
                        & !result.channel_interrupt_flags_raw().value();
                    result.set_channel_interrupt_flags_raw(u7::new(reset));

                    self.dma.interrupt_control = result;
                    self.scheduler.schedule(Event::DmaUpdate, 0);
                }
                io::Reg::Gp0 => {
                    let mut raw = 0u32;
                    value.write_to(&mut raw.as_mut_bytes()[offset..]);
                    self.gpu.render_queue.push_back(raw);

                    self.scheduler.schedule(Event::Gpu, 0);
                    self.scheduler.schedule(Event::DmaUpdate, 0);
                }
                io::Reg::Gp1 => {
                    let mut raw = 0u32;
                    value.write_to(&mut raw.as_mut_bytes()[offset..]);
                    self.gpu.display_queue.push_back(raw);

                    self.scheduler.schedule(Event::Gpu, 0);
                    self.scheduler.schedule(Event::DmaUpdate, 0);
                }
                io::Reg::Cdrom0 | io::Reg::Cdrom1 | io::Reg::Cdrom2 | io::Reg::Cdrom3 => {
                    let mut byte = 0u8;
                    value.write_to(byte.as_mut_bytes());

                    let reg = reg.cdrom_reg().unwrap();
                    self.cdrom
                        .write_queue
                        .push_back(cdrom::RegWrite { reg, value: byte });

                    self.scheduler
                        .schedule(Event::Cdrom(cdrom::Event::Update), 0);
                }
                io::Reg::Timer1Value => {
                    let bytes = self.timers.timer1.value.as_mut_bytes();
                    value.write_to(&mut bytes[offset..]);
                }
                io::Reg::Timer1Mode => {
                    self.timers.timer1.value = 0;

                    let bytes = self.timers.timer1.mode.as_mut_bytes();
                    value.write_to(&mut bytes[offset..]);
                }
                io::Reg::Timer1Target => {
                    let bytes = self.timers.timer1.value.as_mut_bytes();
                    value.write_to(&mut bytes[offset..]);
                }
                io::Reg::Timer2Value => {
                    let bytes = self.timers.timer2.value.as_mut_bytes();
                    value.write_to(&mut bytes[offset..]);
                }
                io::Reg::Timer2Mode => {
                    self.timers.timer2.value = 0;

                    let bytes = self.timers.timer2.mode.as_mut_bytes();
                    value.write_to(&mut bytes[offset..]);
                }
                io::Reg::Timer2Target => {
                    let bytes = self.timers.timer2.value.as_mut_bytes();
                    value.write_to(&mut bytes[offset..]);
                }
                _ => default(),
            };
        } else {
            if !SILENT {
                warn!(
                    self.loggers.bus,
                    "{} bytes written to unknown IO port {}: 0x{:X?}",
                    size_of::<P>(),
                    addr,
                    value,
                );
            }

            default()
        }
    }

    pub fn write_unaligned<P, const SILENT: bool>(&mut self, addr: Address, value: P)
    where
        P: Primitive,
    {
        if let Some(phys) = addr.physical() {
            let Some(region) = phys.region() else {
                if !SILENT {
                    warn!(
                        self.loggers.bus,
                        "write to {addr} ({phys}) which is in an unknown region"
                    );
                }

                return;
            };

            let offset = phys.value() - region.start().value();
            match region {
                Region::Ram => self.memory.ram[offset as usize..].write(value),
                Region::RamMirror => {
                    self.memory.ram[(offset & 0x001F_FFFF) as usize..].write(value);
                }
                Region::Expansion1 => self.memory.expansion_1[offset as usize..].write(value),
                Region::ScratchPad => self.memory.scratchpad[offset as usize..].write(value),
                Region::IOPorts => self.write_io_ports::<P, SILENT>(addr, value),
                Region::Expansion2 => self.memory.expansion_2[offset as usize..].write(value),
                Region::Expansion3 => self.memory.expansion_3[offset as usize..].write(value),
                Region::BIOS => self.memory.bios[offset as usize..].write(value),
            }
        } else {
            self.cpu.cache_control_mut().as_mut_bytes().write(value);
        }
    }

    #[inline(always)]
    pub fn write<P, const SILENT: bool>(
        &mut self,
        addr: Address,
        value: P,
    ) -> Result<(), MisalignedAddressErr>
    where
        P: Primitive,
    {
        (addr.is_aligned(P::ALIGNMENT))
            .then(|| self.write_unaligned::<P, SILENT>(addr, value))
            .ok_or(MisalignedAddressErr {
                addr,
                alignment: P::ALIGNMENT,
            })
    }
}
