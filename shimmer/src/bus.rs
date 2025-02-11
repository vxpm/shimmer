use crate::{PSX, cdrom, scheduler::Event, sio0};
use bitos::integer::u7;
use easyerr::Error;
use shimmer_core::{
    cdrom::RegWrite as CdromRegWrite,
    dma,
    mem::{Address, Primitive, PrimitiveRw, Region, io},
};
use tinylog::{debug, trace, warn};
use zerocopy::IntoBytes;

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
                io::Reg::JoyData => {
                    let data = [self.sio0.read_rx(), 0xFF, 0xFF, 0xFF];

                    self.scheduler.schedule(Event::Sio(sio0::Event::Update), 0);
                    P::read_from_buf(&data[offset..])
                }
                io::Reg::JoyStat => {
                    let bytes = self.sio0.status.as_bytes();

                    self.scheduler.schedule(Event::Sio(sio0::Event::Update), 0);
                    P::read_from_buf(&bytes[offset..])
                }
                io::Reg::JoyMode => {
                    let bytes = self.sio0.mode.as_bytes();

                    self.scheduler.schedule(Event::Sio(sio0::Event::Update), 0);
                    P::read_from_buf(&bytes[offset..])
                }
                io::Reg::JoyControl => {
                    let bytes = self.sio0.control.as_bytes();

                    self.scheduler.schedule(Event::Sio(sio0::Event::Update), 0);
                    P::read_from_buf(&bytes[offset..])
                }
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
            self.cpu.cache_control.as_bytes().read()
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
                let ignore_list = [
                    // spu
                    io::Reg::SramFifo,
                    io::Reg::SpuControl,
                    io::Reg::SpuStatus,
                    // joypad
                    io::Reg::JoyData,
                    io::Reg::JoyControl,
                    io::Reg::JoyMode,
                    io::Reg::JoyStat,
                ];

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
                    let mut data = 0u8;
                    value.write_to(data.as_mut_bytes());

                    let reg = reg.cdrom_reg().unwrap();
                    self.cdrom
                        .write_queue
                        .push_back(CdromRegWrite { reg, value: data });

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
                io::Reg::JoyData => {
                    let mut bytes = [0; 4];
                    value.write_to(&mut bytes[offset..]);

                    self.sio0.tx = Some(bytes[0]);
                    self.scheduler.schedule(Event::Sio(sio0::Event::Update), 0);
                }
                io::Reg::JoyStat => {
                    // read only
                }
                io::Reg::JoyMode => {
                    let bytes = self.sio0.mode.as_mut_bytes();
                    value.write_to(&mut bytes[offset..]);
                    self.scheduler.schedule(Event::Sio(sio0::Event::Update), 0);
                }
                io::Reg::JoyControl => {
                    let bytes = self.sio0.control.as_mut_bytes();
                    value.write_to(&mut bytes[offset..]);
                    self.scheduler.schedule(Event::Sio(sio0::Event::Update), 0);
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
            self.cpu.cache_control.as_mut_bytes().write(value);
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
