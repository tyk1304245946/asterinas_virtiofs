// SPDX-License-Identifier: MPL-2.0

use core::fmt::Debug;

use ostd::Pod;

use super::fuse::*;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct VirtiofsReq {
    // Device-readable part
    in_header: FuseInHeader,
    datain: [u8; 0], // Flexible array members are typically represented as a zero-length array.

    // Device-writable part
    out_header: FuseOutHeader,
    dataout: [u8; 0], // Same as above for the writable part.
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct VirtioFsReadReq {
    // Device-readable part
    in_header: FuseInHeader,
    read_union: ReadUnion,

    // Device-writable part
    out_header: FuseOutHeader,
    dataout: [u8; 0], // Represents a flexible array member
}

// Define the union equivalent in Rust
#[repr(C)]
#[derive(Clone, Copy, Pod)]
pub union ReadUnion {
    readin: FuseReadIn,
    datain: [u8; core::mem::size_of::<FuseReadIn>()],
}

impl Default for ReadUnion {
    fn default() -> Self {
        Self {
            datain: [0u8; core::mem::size_of::<FuseReadIn>()],
        }
    }
}

// todo: Implement Debug for ReadUnion
impl Debug for ReadUnion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ReadUnion")
            // .field("readin", &self.readin)
            // .field("datain", &self.datain)
            .finish()
    }
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct VirtioFsNotify {
    // Device-writable part
    pub out_hdr: FuseOutHeader,
    pub dataout: [u8; 0], // Represents a flexible array member
}
