//! FUSE kernel interface.
//!
//! Types and definitions used for communication between the kernel driver and the userspace
//! part of a FUSE filesystem. Since the kernel driver may be installed independently, the ABI
//! interface is versioned and capabilities are exchanged during the initialization (mounting)
//! of a filesystem.
//!
//! OSXFUSE (macOS): https://github.com/osxfuse/fuse/blob/master/include/fuse_kernel.h
//! - supports ABI 7.8 in OSXFUSE 2.x
//! - supports ABI 7.19 since OSXFUSE 3.0.0
//!
//! libfuse (Linux/BSD): https://github.com/libfuse/libfuse/blob/master/include/fuse_kernel.h
//! - supports ABI 7.8 since FUSE 2.6.0
//! - supports ABI 7.12 since FUSE 2.8.0
//! - supports ABI 7.18 since FUSE 2.9.0
//! - supports ABI 7.19 since FUSE 2.9.1
//! - supports ABI 7.26 since FUSE 3.0.0
//!
//! Items without a version annotation are valid with ABI 7.8 and later

#![warn(missing_debug_implementations, rust_2018_idioms)]
#![allow(missing_docs)]

use core::convert::TryFrom;

use int_to_c_enum::TryFromInt;
use ostd::Pod;

pub const FUSE_KERNEL_VERSION: u32 = 7;

#[cfg(not(feature = "abi-7-9"))]
pub const FUSE_KERNEL_MINOR_VERSION: u32 = 8;
#[cfg(all(feature = "abi-7-9", not(feature = "abi-7-10")))]
pub const FUSE_KERNEL_MINOR_VERSION: u32 = 9;
#[cfg(all(feature = "abi-7-10", not(feature = "abi-7-11")))]
pub const FUSE_KERNEL_MINOR_VERSION: u32 = 10;
#[cfg(all(feature = "abi-7-11", not(feature = "abi-7-12")))]
pub const FUSE_KERNEL_MINOR_VERSION: u32 = 11;
#[cfg(all(feature = "abi-7-12", not(feature = "abi-7-13")))]
pub const FUSE_KERNEL_MINOR_VERSION: u32 = 12;
#[cfg(all(feature = "abi-7-13", not(feature = "abi-7-14")))]
pub const FUSE_KERNEL_MINOR_VERSION: u32 = 13;
#[cfg(all(feature = "abi-7-14", not(feature = "abi-7-15")))]
pub const FUSE_KERNEL_MINOR_VERSION: u32 = 14;
#[cfg(all(feature = "abi-7-15", not(feature = "abi-7-16")))]
pub const FUSE_KERNEL_MINOR_VERSION: u32 = 15;
#[cfg(all(feature = "abi-7-16", not(feature = "abi-7-17")))]
pub const FUSE_KERNEL_MINOR_VERSION: u32 = 16;
#[cfg(all(feature = "abi-7-17", not(feature = "abi-7-18")))]
pub const FUSE_KERNEL_MINOR_VERSION: u32 = 17;
#[cfg(all(feature = "abi-7-18", not(feature = "abi-7-19")))]
pub const FUSE_KERNEL_MINOR_VERSION: u32 = 18;
#[cfg(feature = "abi-7-19")]
pub const FUSE_KERNEL_MINOR_VERSION: u32 = 19;

pub const FUSE_ROOT_ID: u64 = 1;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseAttr {
    pub ino: u64,
    pub size: u64,
    pub blocks: u64,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
    #[cfg(target_os = "macos")]
    pub crtime: u64,
    pub atimensec: u32,
    pub mtimensec: u32,
    pub ctimensec: u32,
    #[cfg(target_os = "macos")]
    pub crtimensec: u32,
    pub mode: u32,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub rdev: u32,
    #[cfg(target_os = "macos")]
    pub flags: u32, // see chflags(2)
    #[cfg(feature = "abi-7-9")]
    pub blksize: u32,
    #[cfg(feature = "abi-7-9")]
    pub padding: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseKstatfs {
    pub blocks: u64,  // Total blocks (in units of frsize)
    pub bfree: u64,   // Free blocks
    pub bavail: u64,  // Free blocks for unprivileged users
    pub files: u64,   // Total inodes
    pub ffree: u64,   // Free inodes
    pub bsize: u32,   // Filesystem block size
    pub namelen: u32, // Maximum filename length
    pub frsize: u32,  // Fundamental file system block size
    pub padding: u32,
    pub spare: [u32; 6],
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseFileLock {
    pub start: u64,
    pub end: u64,
    pub typ: u32,
    pub pid: u32,
}

pub mod consts {
    // Bitmasks for fuse_setattr_in.valid
    pub const FATTR_MODE: u32 = 1 << 0;
    pub const FATTR_UID: u32 = 1 << 1;
    pub const FATTR_GID: u32 = 1 << 2;
    pub const FATTR_SIZE: u32 = 1 << 3;
    pub const FATTR_ATIME: u32 = 1 << 4;
    pub const FATTR_MTIME: u32 = 1 << 5;
    pub const FATTR_FH: u32 = 1 << 6;
    #[cfg(feature = "abi-7-9")]
    pub const FATTR_ATIME_NOW: u32 = 1 << 7;
    #[cfg(feature = "abi-7-9")]
    pub const FATTR_MTIME_NOW: u32 = 1 << 8;
    #[cfg(feature = "abi-7-9")]
    pub const FATTR_LOCKOWNER: u32 = 1 << 9;

    #[cfg(target_os = "macos")]
    pub const FATTR_CRTIME: u32 = 1 << 28;
    #[cfg(target_os = "macos")]
    pub const FATTR_CHGTIME: u32 = 1 << 29;
    #[cfg(target_os = "macos")]
    pub const FATTR_BKUPTIME: u32 = 1 << 30;
    #[cfg(target_os = "macos")]
    pub const FATTR_FLAGS: u32 = 1 << 31;

    // Flags returned by the open request
    pub const FOPEN_DIRECT_IO: u32 = 1 << 0; // bypass page cache for this open file
    pub const FOPEN_KEEP_CACHE: u32 = 1 << 1; // don't invalidate the data cache on open
    #[cfg(feature = "abi-7-10")]
    pub const FOPEN_NONSEEKABLE: u32 = 1 << 2; // the file is not seekable

    #[cfg(target_os = "macos")]
    pub const FOPEN_PURGE_ATTR: u32 = 1 << 30;
    #[cfg(target_os = "macos")]
    pub const FOPEN_PURGE_UBC: u32 = 1 << 31;

    // Init request/reply flags
    pub const FUSE_ASYNC_READ: u32 = 1 << 0; // asynchronous read requests
    pub const FUSE_POSIX_LOCKS: u32 = 1 << 1; // remote locking for POSIX file locks
    #[cfg(feature = "abi-7-9")]
    pub const FUSE_FILE_OPS: u32 = 1 << 2; // kernel sends file handle for fstat, etc...
    #[cfg(feature = "abi-7-9")]
    pub const FUSE_ATOMIC_O_TRUNC: u32 = 1 << 3; // handles the O_TRUNC open flag in the filesystem
    #[cfg(feature = "abi-7-10")]
    pub const FUSE_EXPORT_SUPPORT: u32 = 1 << 4; // filesystem handles lookups of "." and ".."
    #[cfg(feature = "abi-7-9")]
    pub const FUSE_BIG_WRITES: u32 = 1 << 5; // filesystem can handle write size larger than 4kB
    #[cfg(feature = "abi-7-12")]
    pub const FUSE_DONT_MASK: u32 = 1 << 6; // don't apply umask to file mode on create operations

    #[cfg(all(feature = "abi-7-14", not(target_os = "macos")))]
    pub const FUSE_SPLICE_WRITE: u32 = 1 << 7; // kernel supports splice write on the device
    #[cfg(all(feature = "abi-7-14", not(target_os = "macos")))]
    pub const FUSE_SPLICE_MOVE: u32 = 1 << 8; // kernel supports splice move on the device
    #[cfg(not(target_os = "macos"))]
    #[cfg(feature = "abi-7-14")]
    pub const FUSE_SPLICE_READ: u32 = 1 << 9; // kernel supports splice read on the device
    #[cfg(feature = "abi-7-17")]
    pub const FUSE_FLOCK_LOCKS: u32 = 1 << 10; // remote locking for BSD style file locks
    #[cfg(feature = "abi-7-18")]
    pub const FUSE_HAS_IOCTL_DIR: u32 = 1 << 11; // kernel supports ioctl on directories

    #[cfg(target_os = "macos")]
    pub const FUSE_ALLOCATE: u32 = 1 << 27;
    #[cfg(target_os = "macos")]
    pub const FUSE_EXCHANGE_DATA: u32 = 1 << 28;
    #[cfg(target_os = "macos")]
    pub const FUSE_CASE_INSENSITIVE: u32 = 1 << 29;
    #[cfg(target_os = "macos")]
    pub const FUSE_VOL_RENAME: u32 = 1 << 30;
    #[cfg(target_os = "macos")]
    pub const FUSE_XTIMES: u32 = 1 << 31;

    // CUSE init request/reply flags
    #[cfg(feature = "abi-7-12")]
    pub const CUSE_UNRESTRICTED_IOCTL: u32 = 1 << 0; // use unrestricted ioctl

    // Release flags
    pub const FUSE_RELEASE_FLUSH: u32 = 1 << 0;
    #[cfg(feature = "abi-7-17")]
    pub const FUSE_RELEASE_FLOCK_UNLOCK: u32 = 1 << 1;

    // Getattr flags
    #[cfg(feature = "abi-7-9")]
    pub const FUSE_GETATTR_FH: u32 = 1 << 0;

    // Lock flags
    #[cfg(feature = "abi-7-9")]
    pub const FUSE_LK_FLOCK: u32 = 1 << 0;

    // Write flags
    #[cfg(feature = "abi-7-9")]
    pub const FUSE_WRITE_CACHE: u32 = 1 << 0; // delayed write from page cache, file handle is guessed
    #[cfg(feature = "abi-7-9")]
    pub const FUSE_WRITE_LOCKOWNER: u32 = 1 << 1; // lock_owner field is valid

    // Read flags
    #[cfg(feature = "abi-7-9")]
    pub const FUSE_READ_LOCKOWNER: u32 = 1 << 1;

    // IOCTL flags
    #[cfg(feature = "abi-7-11")]
    pub const FUSE_IOCTL_COMPAT: u32 = 1 << 0; // 32bit compat ioctl on 64bit machine
    #[cfg(feature = "abi-7-11")]
    pub const FUSE_IOCTL_UNRESTRICTED: u32 = 1 << 1; // not restricted to well-formed ioctls, retry allowed
    #[cfg(feature = "abi-7-11")]
    pub const FUSE_IOCTL_RETRY: u32 = 1 << 2; // retry with new iovecs
    #[cfg(feature = "abi-7-16")]
    pub const FUSE_IOCTL_32BIT: u32 = 1 << 3; // 32bit ioctl
    #[cfg(feature = "abi-7-18")]
    pub const FUSE_IOCTL_DIR: u32 = 1 << 4; // is a directory
    #[cfg(feature = "abi-7-11")]
    pub const FUSE_IOCTL_MAX_IOV: u32 = 256; // maximum of in_iovecs + out_iovecs

    // Poll flags
    #[cfg(feature = "abi-7-9")]
    pub const FUSE_POLL_SCHEDULE_NOTIFY: u32 = 1 << 0; // request poll notify

    // The read buffer is required to be at least 8k, but may be much larger
    pub const FUSE_MIN_READ_BUFFER: usize = 8192;
}

/// Invalid opcode error.
#[derive(Debug)]
pub struct InvalidOpcodeError;

#[repr(u8)]
#[derive(Default, Debug, Clone, Copy, TryFromInt)]
#[allow(non_camel_case_types)]
pub enum FuseOpcode {
    #[default]
    FuseLookup = 1,
    FuseForget = 2, // no reply
    FuseGetattr = 3,
    FuseSetattr = 4,
    FuseReadlink = 5,
    FuseSymlink = 6,
    FuseMknod = 8,
    FuseMkdir = 9,
    FuseUnlink = 10,
    FuseRmdir = 11,
    FuseRename = 12,
    FuseLink = 13,
    FuseOpen = 14,
    FuseRead = 15,
    FuseWrite = 16,
    FuseStatfs = 17,
    FuseRelease = 18,
    FuseFsync = 20,
    FuseSetxattr = 21,
    FuseGetxattr = 22,
    FuseListxattr = 23,
    FuseRemovexattr = 24,
    FuseFlush = 25,
    FuseInit = 26,
    FuseOpendir = 27,
    FuseReaddir = 28,
    FuseReleasedir = 29,
    FuseFsyncdir = 30,
    FuseGetlk = 31,
    FuseSetlk = 32,
    FuseSetlkw = 33,
    FuseAccess = 34,
    FuseCreate = 35,
    FuseInterrupt = 36,
    FuseBmap = 37,
    FuseDestroy = 38,
    #[cfg(feature = "abi-7-11")]
    FuseIoctl = 39,
    #[cfg(feature = "abi-7-11")]
    FusePoll = 40,
    #[cfg(feature = "abi-7-15")]
    FuseNotifyReply = 41,
    #[cfg(feature = "abi-7-16")]
    FuseBatchForget = 42,
    #[cfg(feature = "abi-7-19")]
    FuseFallocate = 43,

    #[cfg(target_os = "macos")]
    FuseSetvolname = 61,
    #[cfg(target_os = "macos")]
    FuseGetxtimes = 62,
    #[cfg(target_os = "macos")]
    FuseExchange = 63,

    #[cfg(feature = "abi-7-12")]
    CuseInit = 4096,
}

impl TryFrom<u32> for FuseOpcode {
    type Error = InvalidOpcodeError;

    fn try_from(n: u32) -> Result<Self, Self::Error> {
        match n {
            1 => Ok(FuseOpcode::FuseLookup),
            2 => Ok(FuseOpcode::FuseForget),
            3 => Ok(FuseOpcode::FuseGetattr),
            4 => Ok(FuseOpcode::FuseSetattr),
            5 => Ok(FuseOpcode::FuseReadlink),
            6 => Ok(FuseOpcode::FuseSymlink),
            8 => Ok(FuseOpcode::FuseMknod),
            9 => Ok(FuseOpcode::FuseMkdir),
            10 => Ok(FuseOpcode::FuseUnlink),
            11 => Ok(FuseOpcode::FuseRmdir),
            12 => Ok(FuseOpcode::FuseRename),
            13 => Ok(FuseOpcode::FuseLink),
            14 => Ok(FuseOpcode::FuseOpen),
            15 => Ok(FuseOpcode::FuseRead),
            16 => Ok(FuseOpcode::FuseWrite),
            17 => Ok(FuseOpcode::FuseStatfs),
            18 => Ok(FuseOpcode::FuseRelease),
            20 => Ok(FuseOpcode::FuseFsync),
            21 => Ok(FuseOpcode::FuseSetxattr),
            22 => Ok(FuseOpcode::FuseGetxattr),
            23 => Ok(FuseOpcode::FuseListxattr),
            24 => Ok(FuseOpcode::FuseRemovexattr),
            25 => Ok(FuseOpcode::FuseFlush),
            26 => Ok(FuseOpcode::FuseInit),
            27 => Ok(FuseOpcode::FuseOpendir),
            28 => Ok(FuseOpcode::FuseReaddir),
            29 => Ok(FuseOpcode::FuseReleasedir),
            30 => Ok(FuseOpcode::FuseFsyncdir),
            31 => Ok(FuseOpcode::FuseGetlk),
            32 => Ok(FuseOpcode::FuseSetlk),
            33 => Ok(FuseOpcode::FuseSetlkw),
            34 => Ok(FuseOpcode::FuseAccess),
            35 => Ok(FuseOpcode::FuseCreate),
            36 => Ok(FuseOpcode::FuseInterrupt),
            37 => Ok(FuseOpcode::FuseBmap),
            38 => Ok(FuseOpcode::FuseDestroy),
            #[cfg(feature = "abi-7-11")]
            39 => Ok(FuseOpcode::FuseIoctl),
            #[cfg(feature = "abi-7-11")]
            40 => Ok(FuseOpcode::FusePoll),
            #[cfg(feature = "abi-7-15")]
            41 => Ok(FuseOpcode::FuseNotifyReply),
            #[cfg(feature = "abi-7-16")]
            42 => Ok(FuseOpcode::FuseBatchForget),
            #[cfg(feature = "abi-7-19")]
            43 => Ok(FuseOpcode::FuseFallocate),

            #[cfg(target_os = "macos")]
            61 => Ok(FuseOpcode::FuseSetvolname),
            #[cfg(target_os = "macos")]
            62 => Ok(FuseOpcode::FuseGetxtimes),
            #[cfg(target_os = "macos")]
            63 => Ok(FuseOpcode::FuseExchange),

            #[cfg(feature = "abi-7-12")]
            4096 => Ok(FuseOpcode::CuseInit),

            _ => Err(InvalidOpcodeError),
        }
    }
}

/// Invalid notify code error.
#[cfg(feature = "abi-7-11")]
#[derive(Debug)]
pub struct InvalidNotifyCodeError;

#[cfg(feature = "abi-7-11")]
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
#[allow(non_camel_case_types)]
pub enum fuse_notify_code {
    #[cfg(feature = "abi-7-11")]
    FUSE_POLL = 1,
    #[cfg(feature = "abi-7-12")]
    FUSE_NOTIFY_INVAL_INODE = 2,
    #[cfg(feature = "abi-7-12")]
    FUSE_NOTIFY_INVAL_ENTRY = 3,
    #[cfg(feature = "abi-7-15")]
    FUSE_NOTIFY_STORE = 4,
    #[cfg(feature = "abi-7-15")]
    FUSE_NOTIFY_RETRIEVE = 5,
    #[cfg(feature = "abi-7-18")]
    FUSE_NOTIFY_DELETE = 6,
}

#[cfg(feature = "abi-7-11")]
impl TryFrom<u32> for fuse_notify_code {
    type Error = InvalidNotifyCodeError;

    fn try_from(n: u32) -> Result<Self, Self::Error> {
        match n {
            #[cfg(feature = "abi-7-11")]
            1 => Ok(fuse_notify_code::FUSE_POLL),
            #[cfg(feature = "abi-7-12")]
            2 => Ok(fuse_notify_code::FUSE_NOTIFY_INVAL_INODE),
            #[cfg(feature = "abi-7-12")]
            3 => Ok(fuse_notify_code::FUSE_NOTIFY_INVAL_ENTRY),
            #[cfg(feature = "abi-7-15")]
            4 => Ok(fuse_notify_code::FUSE_NOTIFY_STORE),
            #[cfg(feature = "abi-7-15")]
            5 => Ok(fuse_notify_code::FUSE_NOTIFY_RETRIEVE),
            #[cfg(feature = "abi-7-18")]
            6 => Ok(fuse_notify_code::FUSE_NOTIFY_DELETE),

            _ => Err(InvalidNotifyCodeError),
        }
    }
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseEntryOut {
    pub nodeid: u64,
    pub generation: u64,
    pub entry_valid: u64,
    pub attr_valid: u64,
    pub entry_valid_nsec: u32,
    pub attr_valid_nsec: u32,
    pub attr: FuseAttr,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseForgetIn {
    pub nlookup: u64,
}

#[cfg(feature = "abi-7-16")]
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct fuse_forget_one {
    pub nodeid: u64,
    pub nlookup: u64,
}

#[cfg(feature = "abi-7-16")]
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct fuse_batch_forget_in {
    pub count: u32,
    pub dummy: u32,
}

#[cfg(feature = "abi-7-9")]
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct fuse_getattr_in {
    pub getattr_flags: u32,
    pub dummy: u32,
    pub fh: u64,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseAttrOut {
    pub attr_valid: u64,
    pub attr_valid_nsec: u32,
    pub dummy: u32,
    pub attr: FuseAttr,
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct fuse_getxtimes_out {
    pub bkuptime: u64,
    pub crtime: u64,
    pub bkuptimensec: u32,
    pub crtimensec: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseMknodIn {
    pub mode: u32,
    pub rdev: u32,
    #[cfg(feature = "abi-7-12")]
    pub umask: u32,
    #[cfg(feature = "abi-7-12")]
    pub padding: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseMkdirIn {
    pub mode: u32,
    #[cfg(not(feature = "abi-7-12"))]
    pub padding: u32,
    #[cfg(feature = "abi-7-12")]
    pub umask: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct fuse_rename_in {
    pub newdir: u64,
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct fuse_exchange_in {
    pub olddir: u64,
    pub newdir: u64,
    pub options: u64,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct fuse_link_in {
    pub oldnodeid: u64,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseSetattrIn {
    pub valid: u32,
    pub padding: u32,
    pub fh: u64,
    pub size: u64,
    #[cfg(not(feature = "abi-7-9"))]
    pub unused1: u64,
    #[cfg(feature = "abi-7-9")]
    pub lock_owner: u64,
    pub atime: u64,
    pub mtime: u64,
    pub unused2: u64,
    pub atimensec: u32,
    pub mtimensec: u32,
    pub unused3: u32,
    pub mode: u32,
    pub unused4: u32,
    pub uid: u32,
    pub gid: u32,
    pub unused5: u32,
    #[cfg(target_os = "macos")]
    pub bkuptime: u64,
    #[cfg(target_os = "macos")]
    pub chgtime: u64,
    #[cfg(target_os = "macos")]
    pub crtime: u64,
    #[cfg(target_os = "macos")]
    pub bkuptimensec: u32,
    #[cfg(target_os = "macos")]
    pub chgtimensec: u32,
    #[cfg(target_os = "macos")]
    pub crtimensec: u32,
    #[cfg(target_os = "macos")]
    pub flags: u32, // see chflags(2)
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseOpenIn {
    pub flags: u32,
    pub unused: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseCreateIn {
    pub flags: u32,
    pub mode: u32,
    #[cfg(feature = "abi-7-12")]
    pub umask: u32,
    #[cfg(feature = "abi-7-12")]
    pub padding: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseOpenOut {
    pub fh: u64,
    pub open_flags: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseReleaseIn {
    pub fh: u64,
    pub flags: u32,
    pub release_flags: u32,
    pub lock_owner: u64,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseFlushIn {
    pub fh: u64,
    pub unused: u32,
    pub padding: u32,
    pub lock_owner: u64,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseReadIn {
    pub fh: u64,
    pub offset: u64,
    pub size: u32,
    #[cfg(feature = "abi-7-9")]
    pub read_flags: u32,
    #[cfg(feature = "abi-7-9")]
    pub lock_owner: u64,
    #[cfg(feature = "abi-7-9")]
    pub flags: u32,
    #[cfg(feature = "abi-7-9")]
    pub padding: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseWriteIn {
    pub fh: u64,
    pub offset: u64,
    pub size: u32,
    pub write_flags: u32,
    #[cfg(feature = "abi-7-9")]
    pub lock_owner: u64,
    #[cfg(feature = "abi-7-9")]
    pub flags: u32,
    #[cfg(feature = "abi-7-9")]
    pub padding: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseWriteOut {
    pub size: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseStatfsOut {
    pub st: FuseKstatfs,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseFsyncIn {
    pub fh: u64,
    pub fsync_flags: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseSetxattrIn {
    pub size: u32,
    pub flags: u32,
    #[cfg(target_os = "macos")]
    pub position: u32,
    #[cfg(target_os = "macos")]
    pub padding: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseGetxattrIn {
    pub size: u32,
    pub padding: u32,
    #[cfg(target_os = "macos")]
    pub position: u32,
    #[cfg(target_os = "macos")]
    pub padding2: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseGetxattrOut {
    pub size: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseLkIn {
    pub fh: u64,
    pub owner: u64,
    pub lk: FuseFileLock,
    #[cfg(feature = "abi-7-9")]
    pub lk_flags: u32,
    #[cfg(feature = "abi-7-9")]
    pub padding: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseLkOut {
    pub lk: FuseFileLock,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseAccessIn {
    pub mask: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseInitIn {
    pub major: u32,
    pub minor: u32,
    pub max_readahead: u32,
    pub flags: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseInitOut {
    pub major: u32,
    pub minor: u32,
    pub max_readahead: u32,
    pub flags: u32,
    #[cfg(not(feature = "abi-7-13"))]
    pub unused: u32,
    #[cfg(feature = "abi-7-13")]
    pub max_background: u16,
    #[cfg(feature = "abi-7-13")]
    pub congestion_threshold: u16,
    pub max_write: u32,
}

#[cfg(feature = "abi-7-12")]
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct cuse_init_in {
    pub major: u32,
    pub minor: u32,
    pub unused: u32,
    pub flags: u32,
}

#[cfg(feature = "abi-7-12")]
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct cuse_init_out {
    pub major: u32,
    pub minor: u32,
    pub unused: u32,
    pub flags: u32,
    pub max_read: u32,
    pub max_write: u32,
    pub dev_major: u32, // chardev major
    pub dev_minor: u32, // chardev minor
    pub spare: [u32; 10],
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseInterruptIn {
    pub unique: u64,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseBmapIn {
    pub block: u64,
    pub blocksize: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseBmapOut {
    pub block: u64,
}

#[cfg(feature = "abi-7-11")]
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct fuse_ioctl_in {
    pub fh: u64,
    pub flags: u32,
    pub cmd: u32,
    pub arg: u64,
    pub in_size: u32,
    pub out_size: u32,
}

#[cfg(feature = "abi-7-16")]
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct fuse_ioctl_iovec {
    pub base: u64,
    pub len: u64,
}

#[cfg(feature = "abi-7-11")]
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct fuse_ioctl_out {
    pub result: i32,
    pub flags: u32,
    pub in_iovs: u32,
    pub out_iovs: u32,
}

#[cfg(feature = "abi-7-11")]
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct fuse_poll_in {
    pub fh: u64,
    pub kh: u64,
    pub flags: u32,
    pub padding: u32,
}

#[cfg(feature = "abi-7-11")]
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct fuse_poll_out {
    pub revents: u32,
    pub padding: u32,
}

#[cfg(feature = "abi-7-11")]
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct fuse_notify_poll_wakeup_out {
    pub kh: u64,
}

#[cfg(feature = "abi-7-19")]
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct fuse_fallocate_in {
    fh: u64,
    offset: u64,
    length: u64,
    mode: u32,
    padding: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseInHeader {
    pub len: u32,
    pub opcode: u32,
    pub unique: u64,
    pub nodeid: u64,
    pub uid: u32,
    pub gid: u32,
    pub pid: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseOutHeader {
    pub len: u32,
    pub error: i32,
    pub unique: u64,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseDirent {
    pub ino: u64,
    pub off: u64,
    pub namelen: u32,
    pub typ: u32,
    // followed by name of namelen bytes
}

#[cfg(feature = "abi-7-12")]
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct fuse_notify_inval_inode_out {
    pub ino: u64,
    pub off: i64,
    pub len: i64,
}

#[cfg(feature = "abi-7-12")]
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct fuse_notify_inval_entry_out {
    pub parent: u64,
    pub namelen: u32,
    pub padding: u32,
}

#[cfg(feature = "abi-7-18")]
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct fuse_notify_delete_out {
    parent: u64,
    child: u64,
    namelen: u32,
    padding: u32,
}

#[cfg(feature = "abi-7-15")]
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct fuse_notify_store_out {
    pub nodeid: u64,
    pub offset: u64,
    pub size: u32,
    pub padding: u32,
}

#[cfg(feature = "abi-7-15")]
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct fuse_notify_retrieve_out {
    pub notify_unique: u64,
    pub nodeid: u64,
    pub offset: u64,
    pub size: u32,
    pub padding: u32,
}

#[cfg(feature = "abi-7-15")]
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct fuse_notify_retrieve_in {
    // matches the size of fuse_write_in
    pub dummy1: u64,
    pub offset: u64,
    pub size: u32,
    pub dummy2: u32,
    pub dummy3: u64,
    pub dummy4: u64,
}
