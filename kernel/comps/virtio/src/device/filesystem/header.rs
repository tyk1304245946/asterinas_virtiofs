// SPDX-License-Identifier: MPL-2.0

use ostd::Pod;

/// FUSE request unique ID flag
///
/// Indicates whether this is a resend request. The receiver should handle this
/// request accordingly.
pub const FUSE_UNIQUE_RESEND: u64 = 1 << 63;

/// This value will be set by the kernel to
/// `(FuseInHeader).{uid, gid}` fields in
/// case when:
/// - FUSE daemon enabled FUSE_ALLOW_IDMAP
/// - idmapping information is not available, and uid/gid
///   cannot be mapped in accordance with an idmapping.
///
/// Note: Idmapping information is always available
/// for inode creation operations like:
/// FUSE_MKNOD, FUSE_SYMLINK, FUSE_MKDIR, FUSE_TMPFILE,
/// FUSE_CREATE, and FUSE_RENAME2 (with RENAME_WHITEOUT).
pub const FUSE_INVALID_UIDGID: u32 = u32::MAX;

/// VirtioNet header precedes each packet
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseInHeader {
    len: u32,
    opcode: u32,
    unique: u64,
    nodeid: u64,
    uid: u32,
    gid: u32,
    pid: u32,
    total_extlen: u16, // length of extensions in 8-byte units
    padding: u16,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseOutHeader {
    len: u32,
    error: i32,
    unique: u64,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseDirent {
    ino: u64,
    off: u64,
    namelen: u32,
    dirent_type: u32,
    name: [u8; 0], // Flexible array member equivalent
}