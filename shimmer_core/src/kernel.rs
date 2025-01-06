#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Function {
    AddCDROMDevice,
    AddDrv,
    AddMemCardDevice,
    AddNullconDriver,
    AdjustA0Table,
    AllocKernelMemory,
    BZero,
    ChangeClearPAD,
    ChangeClearRCnt,
    CharToUpper,
    CloseEvent,
    DequeueCdIntr,
    EnableEvent,
    EnqueueSyscallHandler,
    EnqueueTimerAndVblankIrqs,
    FlushCache,
    HookEntryInt,
    InitDefInt,
    InitHeap,
    InstallDevices,
    InstallExceptionHandlers,
    Memcpy,
    OpenEvent,
    Printf,
    PutChar,
    Remove96,
    ResetEntryInt,
    ReturnFromException,
    SendGpuCommandWord,
    SetJmp,
    Strcmp,
    Strlen,
    Strncat,
    SysDeqIntRP,
    SysInitMemory,
    Write,
}

impl Function {
    pub fn a0(code: u8) -> Option<Self> {
        Some(match code {
            0x03 => Self::Write,
            0x13 => Self::SetJmp,
            0x16 => Self::Strncat,
            0x17 => Self::Strcmp,
            0x1B => Self::Strlen,
            0x25 => Self::CharToUpper,
            0x28 => Self::BZero,
            0x2A => Self::Memcpy,
            0x39 => Self::InitHeap,
            0x3B => Self::PutChar,
            0x3F => Self::Printf,
            0x44 => Self::FlushCache,
            0x49 => Self::SendGpuCommandWord,
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
            0x08 => Self::OpenEvent,
            0x09 => Self::CloseEvent,
            0x0C => Self::EnableEvent,
            0x17 => Self::ReturnFromException,
            0x18 => Self::ResetEntryInt,
            0x19 => Self::HookEntryInt,
            0x35 => Self::Write,
            0x3D => Self::PutChar,
            0x47 => Self::AddDrv,
            0x5B => Self::ChangeClearPAD,
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
            0x0A => Self::ChangeClearRCnt,
            0x0C => Self::InitDefInt,
            0x12 => Self::InstallDevices,
            0x1C => Self::AdjustA0Table,
            _ => return None,
        })
    }

    pub fn args(&self) -> usize {
        match self {
            Self::AddDrv => 1,
            Self::AllocKernelMemory => 1,
            Self::BZero => 2,
            Self::ChangeClearPAD => 1,
            Self::ChangeClearRCnt => 2,
            Self::CharToUpper => 1,
            Self::CloseEvent => 1,
            Self::EnableEvent => 1,
            Self::EnqueueSyscallHandler => 1,
            Self::EnqueueTimerAndVblankIrqs => 1,
            Self::HookEntryInt => 1,
            Self::InitDefInt => 1,
            Self::InitHeap => 2,
            Self::InstallDevices => 1,
            Self::Memcpy => 3,
            Self::OpenEvent => 4,
            Self::Printf => 4,
            Self::PutChar => 1,
            Self::SendGpuCommandWord => 1,
            Self::SetJmp => 1,
            Self::Strcmp => 2,
            Self::Strlen => 1,
            Self::Strncat => 3,
            Self::SysDeqIntRP => 2,
            Self::SysInitMemory => 2,
            Self::Write => 3,
            _ => 0,
        }
    }
}
