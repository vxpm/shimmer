#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Function {
    PutChar,
    Strlen,
    Printf,
    InitHeap,
    SysDeqIntRP,
    Write,
    DequeueCdIntr,
    CloseEvent,
    Remove96,
    EnqueueTimerAndVblankIrqs,
    AllocKernelMemory,
    EnqueueSyscallHandler,
    SysInitMemory,
    FlushCache,
    Memcpy,
    AddDrv,
    AddMemCardDevice,
    AddCDROMDevice,
    Strcmp,
    CharToUpper,
    InitDefInt,
    AddNullconDriver,
    InstallDevices,
    ResetEntryInt,
    InstallExceptionHandlers,
    AdjustA0Table,
}

impl Function {
    pub fn a0(code: u8) -> Option<Self> {
        Some(match code {
            0x03 => Self::Write,
            0x17 => Self::Strcmp,
            0x1B => Self::Strlen,
            0x25 => Self::CharToUpper,
            0x2A => Self::Memcpy,
            0x39 => Self::InitHeap,
            0x3B => Self::PutChar,
            0x3F => Self::Printf,
            0x44 => Self::FlushCache,
            0x56 | 0x72 => Self::Remove96,
            0x96 => Self::AddCDROMDevice,
            0x97 => Self::AddMemCardDevice,
            0x99 => Self::AddNullconDriver,
            0xA3 => Self::DequeueCdIntr,
            _ => return None,
        })
    }

    pub fn b0(code: u8) -> Option<Self> {
        Some(match code {
            0x00 => Self::AllocKernelMemory,
            0x09 => Self::CloseEvent,
            0x18 => Self::ResetEntryInt,
            0x35 => Self::Write,
            0x3D => Self::PutChar,
            0x47 => Self::AddDrv,
            _ => return None,
        })
    }

    pub fn c0(code: u8) -> Option<Self> {
        Some(match code {
            0x00 => Self::EnqueueTimerAndVblankIrqs,
            0x01 => Self::EnqueueSyscallHandler,
            0x03 => Self::SysDeqIntRP,
            0x07 => Self::InstallExceptionHandlers,
            0x08 => Self::SysInitMemory,
            0x0C => Self::InitDefInt,
            0x12 => Self::InstallDevices,
            0x1C => Self::AdjustA0Table,
            _ => return None,
        })
    }
}
