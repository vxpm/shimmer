//! Items related to the executable format of the PSX.

use crate::mem::Address;
use binrw::BinRead;
use std::ffi::{CStr, CString};

/// Header of a PSX executable.
#[derive(Debug, Clone, BinRead)]
#[br(magic = b"PS-X EXE\0\0\0\0\0\0\0\0")]
pub struct Header {
    pub initial_pc: Address,
    pub initial_gp: u32,

    pub destination: Address,
    pub length: u32,

    pub data_start: Address,
    pub data_length: u32,

    pub bss_start: Address,
    pub bss_length: u32,

    pub initial_sp_base: u32,
    pub initial_sp_offset: u32,

    #[br(pad_before = 20, count = 0x7B4, try_map = |x: Vec<u8>| CStr::from_bytes_until_nul(&x).map(|x| x.to_owned()))]
    pub marker: CString,
}

/// A PSX executable.
#[derive(Debug, Clone, BinRead)]
pub struct Executable {
    pub header: Header,
    #[br(count = header.length)]
    pub program: Vec<u8>,
}
