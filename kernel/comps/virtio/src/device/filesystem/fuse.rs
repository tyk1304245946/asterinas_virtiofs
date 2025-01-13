//! FUSE kernel interface.
//! //! libfuse (Linux/BSD): https://github.com/libfuse/libfuse/blob/master/include/fuse_kernel.h
//!
//!
//! /* SPDX-License-Identifier: ((GPL-2.0 WITH Linux-syscall-note) OR BSD-2-Clause) */
/*
    This file defines the kernel interface of FUSE
    Copyright (C) 2001-2008  Miklos Szeredi <miklos@szeredi.hu>

    This program can be distributed under the terms of the GNU GPL.
    See the file COPYING.

    This -- and only this -- header file may also be distributed under
    the terms of the BSD Licence as follows:

    Copyright (C) 2001-2007 Miklos Szeredi. All rights reserved.

    Redistribution and use in source and binary forms, with or without
    modification, are permitted provided that the following conditions
    are met:
    1. Redistributions of source code must retain the above copyright
       notice, this list of conditions and the following disclaimer.
    2. Redistributions in binary form must reproduce the above copyright
       notice, this list of conditions and the following disclaimer in the
       documentation and/or other materials provided with the distribution.

    THIS SOFTWARE IS PROVIDED BY AUTHOR AND CONTRIBUTORS ``AS IS'' AND
    ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
    IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
    ARE DISCLAIMED.  IN NO EVENT SHALL AUTHOR OR CONTRIBUTORS BE LIABLE
    FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
    DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
    OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
    HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
    LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
    OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
    SUCH DAMAGE.
*/

/*
 * This file defines the kernel interface of FUSE
 *
 * Protocol changelog:
 *
 * 7.1:
 *  - add the following messages:
 *      FUSE_SETATTR, FUSE_SYMLINK, FUSE_MKNOD, FUSE_MKDIR, FUSE_UNLINK,
 *      FUSE_RMDIR, FUSE_RENAME, FUSE_LINK, FUSE_OPEN, FUSE_READ, FUSE_WRITE,
 *      FUSE_RELEASE, FUSE_FSYNC, FUSE_FLUSH, FUSE_SETXATTR, FUSE_GETXATTR,
 *      FUSE_LISTXATTR, FUSE_REMOVEXATTR, FUSE_OPENDIR, FUSE_READDIR,
 *      FUSE_RELEASEDIR
 *  - add padding to messages to accommodate 32-bit servers on 64-bit kernels
 *
 * 7.2:
 *  - add FOPEN_DIRECT_IO and FOPEN_KEEP_CACHE flags
 *  - add FUSE_FSYNCDIR message
 *
 * 7.3:
 *  - add FUSE_ACCESS message
 *  - add FUSE_CREATE message
 *  - add filehandle to fuse_setattr_in
 *
 * 7.4:
 *  - add frsize to fuse_kstatfs
 *  - clean up request size limit checking
 *
 * 7.5:
 *  - add flags and max_write to fuse_init_out
 *
 * 7.6:
 *  - add max_readahead to fuse_init_in and fuse_init_out
 *
 * 7.7:
 *  - add FUSE_INTERRUPT message
 *  - add POSIX file lock support
 *
 * 7.8:
 *  - add lock_owner and flags fields to fuse_release_in
 *  - add FUSE_BMAP message
 *  - add FUSE_DESTROY message
 *
 * 7.9:
 *  - new fuse_getattr_in input argument of GETATTR
 *  - add lk_flags in fuse_lk_in
 *  - add lock_owner field to fuse_setattr_in, fuse_read_in and fuse_write_in
 *  - add blksize field to fuse_attr
 *  - add file flags field to fuse_read_in and fuse_write_in
 *  - Add ATIME_NOW and MTIME_NOW flags to fuse_setattr_in
 *
 * 7.10
 *  - add nonseekable open flag
 *
 * 7.11
 *  - add IOCTL message
 *  - add unsolicited notification support
 *  - add POLL message and NOTIFY_POLL notification
 *
 * 7.12
 *  - add umask flag to input argument of create, mknod and mkdir
 *  - add notification messages for invalidation of inodes and
 *    directory entries
 *
 * 7.13
 *  - make max number of background requests and congestion threshold
 *    tunables
 *
 * 7.14
 *  - add splice support to fuse device
 *
 * 7.15
 *  - add store notify
 *  - add retrieve notify
 *
 * 7.16
 *  - add BATCH_FORGET request
 *  - FUSE_IOCTL_UNRESTRICTED shall now return with array of 'struct
 *    fuse_ioctl_iovec' instead of ambiguous 'struct iovec'
 *  - add FUSE_IOCTL_32BIT flag
 *
 * 7.17
 *  - add FUSE_FLOCK_LOCKS and FUSE_RELEASE_FLOCK_UNLOCK
 *
 * 7.18
 *  - add FUSE_IOCTL_DIR flag
 *  - add FUSE_NOTIFY_DELETE
 *
 * 7.19
 *  - add FUSE_FALLOCATE
 *
 * 7.20
 *  - add FUSE_AUTO_INVAL_DATA
 *
 * 7.21
 *  - add FUSE_READDIRPLUS
 *  - send the requested events in POLL request
 *
 * 7.22
 *  - add FUSE_ASYNC_DIO
 *
 * 7.23
 *  - add FUSE_WRITEBACK_CACHE
 *  - add time_gran to fuse_init_out
 *  - add reserved space to fuse_init_out
 *  - add FATTR_CTIME
 *  - add ctime and ctimensec to fuse_setattr_in
 *  - add FUSE_RENAME2 request
 *  - add FUSE_NO_OPEN_SUPPORT flag
 *
 *  7.24
 *  - add FUSE_LSEEK for SEEK_HOLE and SEEK_DATA support
 *
 *  7.25
 *  - add FUSE_PARALLEL_DIROPS
 *
 *  7.26
 *  - add FUSE_HANDLE_KILLPRIV
 *  - add FUSE_POSIX_ACL
 *
 *  7.27
 *  - add FUSE_ABORT_ERROR
 *
 *  7.28
 *  - add FUSE_COPY_FILE_RANGE
 *  - add FOPEN_CACHE_DIR
 *  - add FUSE_MAX_PAGES, add max_pages to init_out
 *  - add FUSE_CACHE_SYMLINKS
 *
 *  7.29
 *  - add FUSE_NO_OPENDIR_SUPPORT flag
 *
 *  7.30
 *  - add FUSE_EXPLICIT_INVAL_DATA
 *  - add FUSE_IOCTL_COMPAT_X32
 *
 *  7.31
 *  - add FUSE_WRITE_KILL_PRIV flag
 *  - add FUSE_SETUPMAPPING and FUSE_REMOVEMAPPING
 *  - add map_alignment to fuse_init_out, add FUSE_MAP_ALIGNMENT flag
 *
 *  7.32
 *  - add flags to fuse_attr, add FUSE_ATTR_SUBMOUNT, add FUSE_SUBMOUNTS
 *
 *  7.33
 *  - add FUSE_HANDLE_KILLPRIV_V2, FUSE_WRITE_KILL_SUIDGID, FATTR_KILL_SUIDGID
 *  - add FUSE_OPEN_KILL_SUIDGID
 *  - extend fuse_setxattr_in, add FUSE_SETXATTR_EXT
 *  - add FUSE_SETXATTR_ACL_KILL_SGID
 *
 *  7.34
 *  - add FUSE_SYNCFS
 *
 *  7.35
 *  - add FOPEN_NOFLUSH
 *
 *  7.36
 *  - extend fuse_init_in with reserved fields, add FUSE_INIT_EXT init flag
 *  - add flags2 to fuse_init_in and fuse_init_out
 *  - add FUSE_SECURITY_CTX init flag
 *  - add security context to create, mkdir, symlink, and mknod requests
 *  - add FUSE_HAS_INODE_DAX, FUSE_ATTR_DAX
 *
 *  7.37
 *  - add FUSE_TMPFILE
 *
 *  7.38
 *  - add FUSE_EXPIRE_ONLY flag to fuse_notify_inval_entry
 *  - add FOPEN_PARALLEL_DIRECT_WRITES
 *  - add total_extlen to fuse_in_header
 *  - add FUSE_MAX_NR_SECCTX
 *  - add extension header
 *  - add FUSE_EXT_GROUPS
 *  - add FUSE_CREATE_SUPP_GROUP
 *  - add FUSE_HAS_EXPIRE_ONLY
 *
 *  7.39
 *  - add FUSE_DIRECT_IO_ALLOW_MMAP
 *  - add FUSE_STATX and related structures
 *
 *  7.40
 *  - add max_stack_depth to fuse_init_out, add FUSE_PASSTHROUGH init flag
 *  - add backing_id to fuse_open_out, add FOPEN_PASSTHROUGH open flag
 *  - add FUSE_NO_EXPORT_SUPPORT init flag
 *  - add FUSE_NOTIFY_RESEND, add FUSE_HAS_RESEND init flag
 *  7.41
 *  - add FUSE_ALLOW_IDMAP
 */

use core::convert::TryFrom;

use bitflags;
use int_to_c_enum::TryFromInt;
use ostd::Pod;

/** Version number of this interface */
pub const FUSE_KERNEL_VERSION: u32 = 7;
/** Minor version number of this interface */
pub const FUSE_KERNEL_MINOR_VERSION: u32 = 40;

/** The node ID of the root inode */
pub const FUSE_ROOT_ID: u64 = 1;

/* Make sure all structures are padded to 64bit boundary, so 32bit
userspace works under 64bit kernels */

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseAttr {
    pub ino: u64,
    pub size: u64,
    pub blocks: u64,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
    pub atimensec: u32,
    pub mtimensec: u32,
    pub ctimensec: u32,
    pub mode: u32,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub rdev: u32,
    pub blksize: u32,
    pub flags: u32,
}

// /*
//  * The following structures are bit-for-bit compatible with the statx(2) ABI in
//  * Linux.
//  */
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseSxTime {
    pub tv_sec: i64,
    pub tv_nsec: u32,
    pub __reserved: i32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseStatx {
    pub mask: u32,
    pub blksize: u32,
    pub attributes: u64,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub mode: u16,
    pub __spare0: [u16; 1],
    pub ino: u64,
    pub size: u64,
    pub blocks: u64,
    pub attributes_mask: u64,
    pub atime: FuseSxTime,
    pub btime: FuseSxTime,
    pub ctime: FuseSxTime,
    pub mtime: FuseSxTime,
    pub rdev_major: u32,
    pub rdev_minor: u32,
    pub dev_major: u32,
    pub dev_minor: u32,
    pub __spare2: [u64; 14],
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseKstatfs {
    pub blocks: u64,
    pub bfree: u64,
    pub bavail: u64,
    pub files: u64,
    pub ffree: u64,
    pub bsize: u32,
    pub namelen: u32,
    pub frsize: u32,
    pub padding: u32,
    pub spare: [u32; 6],
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseFileLock {
    pub start: u64,
    pub end: u64,
    pub type_: u32,
    pub pid: u32, // tgid
}

// /**
//  * Bitmasks for fuse_setattr_in.valid
//  */
pub const FATTR_MODE: u32 = 1 << 0;
pub const FATTR_UID: u32 = 1 << 1;
pub const FATTR_GID: u32 = 1 << 2;
pub const FATTR_SIZE: u32 = 1 << 3;
pub const FATTR_ATIME: u32 = 1 << 4;
pub const FATTR_MTIME: u32 = 1 << 5;
pub const FATTR_FH: u32 = 1 << 6;
pub const FATTR_ATIME_NOW: u32 = 1 << 7;
pub const FATTR_MTIME_NOW: u32 = 1 << 8;
pub const FATTR_LOCKOWNER: u32 = 1 << 9;
pub const FATTR_CTIME: u32 = 1 << 10;
pub const FATTR_KILL_SUIDGID: u32 = 1 << 11;

//TODO: bitflags

/**
 * Flags returned by the OPEN request
 *
 * FOPEN_DIRECT_IO: bypass page cache for this open file
 * FOPEN_KEEP_CACHE: don't invalidate the data cache on open
 * FOPEN_NONSEEKABLE: the file is not seekable
 * FOPEN_CACHE_DIR: allow caching this directory
 * FOPEN_STREAM: the file is stream-like (no file position at all)
 * FOPEN_NOFLUSH: don't flush data cache on close (unless FUSE_WRITEBACK_CACHE)
 * FOPEN_PARALLEL_DIRECT_WRITES: Allow concurrent direct writes on the same inode
 * FOPEN_PASSTHROUGH: passthrough read/write io for this open file
 */

pub const FOPEN_DIRECT_IO: u32 = 1 << 0;
pub const FOPEN_KEEP_CACHE: u32 = 1 << 1;
pub const FOPEN_NONSEEKABLE: u32 = 1 << 2;
pub const FOPEN_CACHE_DIR: u32 = 1 << 3;
pub const FOPEN_STREAM: u32 = 1 << 4;
pub const FOPEN_NOFLUSH: u32 = 1 << 5;
pub const FOPEN_PARALLEL_DIRECT_WRITES: u32 = 1 << 6;
pub const FOPEN_PASSTHROUGH: u32 = 1 << 7;

/**
 * INIT request/reply flags
 *
 * FUSE_ASYNC_READ: asynchronous read requests
 * FUSE_POSIX_LOCKS: remote locking for POSIX file locks
 * FUSE_FILE_OPS: kernel sends file handle for fstat, etc... (not yet supported)
 * FUSE_ATOMIC_O_TRUNC: handles the O_TRUNC open flag in the filesystem
 * FUSE_EXPORT_SUPPORT: filesystem handles lookups of "." and ".."
 * FUSE_BIG_WRITES: filesystem can handle write size larger than 4kB
 * FUSE_DONT_MASK: don't apply umask to file mode on create operations
 * FUSE_SPLICE_WRITE: kernel supports splice write on the device
 * FUSE_SPLICE_MOVE: kernel supports splice move on the device
 * FUSE_SPLICE_READ: kernel supports splice read on the device
 * FUSE_FLOCK_LOCKS: remote locking for BSD style file locks
 * FUSE_HAS_IOCTL_DIR: kernel supports ioctl on directories
 * FUSE_AUTO_INVAL_DATA: automatically invalidate cached pages
 * FUSE_DO_READDIRPLUS: do READDIRPLUS (READDIR+LOOKUP in one)
 * FUSE_READDIRPLUS_AUTO: adaptive readdirplus
 * FUSE_ASYNC_DIO: asynchronous direct I/O submission
 * FUSE_WRITEBACK_CACHE: use writeback cache for buffered writes
 * FUSE_NO_OPEN_SUPPORT: kernel supports zero-message opens
 * FUSE_PARALLEL_DIROPS: allow parallel lookups and readdir
 * FUSE_HANDLE_KILLPRIV: fs handles killing suid/sgid/cap on write/chown/trunc
 * FUSE_POSIX_ACL: filesystem supports posix acls
 * FUSE_ABORT_ERROR: reading the device after abort returns ECONNABORTED
 * FUSE_MAX_PAGES: init_out.max_pages contains the max number of req pages
 * FUSE_CACHE_SYMLINKS: cache READLINK responses
 * FUSE_NO_OPENDIR_SUPPORT: kernel supports zero-message opendir
 * FUSE_EXPLICIT_INVAL_DATA: only invalidate cached pages on explicit request
 * FUSE_MAP_ALIGNMENT: init_out.map_alignment contains log2(byte alignment) for
 *		       foffset and moffset fields in struct
 *		       fuse_setupmapping_out and fuse_removemapping_one.
 * FUSE_SUBMOUNTS: kernel supports auto-mounting directory submounts
 * FUSE_HANDLE_KILLPRIV_V2: fs kills suid/sgid/cap on write/chown/trunc.
 *			Upon write/truncate suid/sgid is only killed if caller
 *			does not have CAP_FSETID. Additionally upon
 *			write/truncate sgid is killed only if file has group
 *			execute permission. (Same as Linux VFS behavior).
 * FUSE_SETXATTR_EXT:	Server supports extended struct fuse_setxattr_in
 * FUSE_INIT_EXT: extended fuse_init_in request
 * FUSE_INIT_RESERVED: reserved, do not use
 * FUSE_SECURITY_CTX:	add security context to create, mkdir, symlink, and
 *			mknod
 * FUSE_HAS_INODE_DAX:  use per inode DAX
 * FUSE_CREATE_SUPP_GROUP: add supplementary group info to create, mkdir,
 *			symlink and mknod (single group that matches parent)
 * FUSE_HAS_EXPIRE_ONLY: kernel supports expiry-only entry invalidation
 * FUSE_DIRECT_IO_ALLOW_MMAP: allow shared mmap in FOPEN_DIRECT_IO mode.
 * FUSE_NO_EXPORT_SUPPORT: explicitly disable export support
 * FUSE_HAS_RESEND: kernel supports resending pending requests, and the high bit
 *		    of the request ID indicates resend requests
 * FUSE_ALLOW_IDMAP: allow creation of idmapped mounts
 */

pub const FUSE_ASYNC_READ: u64 = 1 << 0;
pub const FUSE_POSIX_LOCKS: u64 = 1 << 1;
pub const FUSE_FILE_OPS: u64 = 1 << 2;
pub const FUSE_ATOMIC_O_TRUNC: u64 = 1 << 3;
pub const FUSE_EXPORT_SUPPORT: u64 = 1 << 4;
pub const FUSE_BIG_WRITES: u64 = 1 << 5;
pub const FUSE_DONT_MASK: u64 = 1 << 6;
pub const FUSE_SPLICE_WRITE: u64 = 1 << 7;
pub const FUSE_SPLICE_MOVE: u64 = 1 << 8;
pub const FUSE_SPLICE_READ: u64 = 1 << 9;
pub const FUSE_FLOCK_LOCKS: u64 = 1 << 10;
pub const FUSE_HAS_IOCTL_DIR: u64 = 1 << 11;
pub const FUSE_AUTO_INVAL_DATA: u64 = 1 << 12;
pub const FUSE_DO_READDIRPLUS: u64 = 1 << 13;
pub const FUSE_READDIRPLUS_AUTO: u64 = 1 << 14;
pub const FUSE_ASYNC_DIO: u64 = 1 << 15;
pub const FUSE_WRITEBACK_CACHE: u64 = 1 << 16;
pub const FUSE_NO_OPEN_SUPPORT: u64 = 1 << 17;
pub const FUSE_PARALLEL_DIROPS: u64 = 1 << 18;
pub const FUSE_HANDLE_KILLPRIV: u64 = 1 << 19;
pub const FUSE_POSIX_ACL: u64 = 1 << 20;
pub const FUSE_ABORT_ERROR: u64 = 1 << 21;
pub const FUSE_MAX_PAGES: u64 = 1 << 22;
pub const FUSE_CACHE_SYMLINKS: u64 = 1 << 23;
pub const FUSE_NO_OPENDIR_SUPPORT: u64 = 1 << 24;
pub const FUSE_EXPLICIT_INVAL_DATA: u64 = 1 << 25;
pub const FUSE_MAP_ALIGNMENT: u64 = 1 << 26;
pub const FUSE_SUBMOUNTS: u64 = 1 << 27;
pub const FUSE_HANDLE_KILLPRIV_V2: u64 = 1 << 28;
pub const FUSE_SETXATTR_EXT: u64 = 1 << 29;
pub const FUSE_INIT_EXT: u64 = 1 << 30;
pub const FUSE_INIT_RESERVED: u64 = 1 << 31;
/* bits 32..63 get shifted down 32 bits into the flags2 field */
pub const FUSE_SECURITY_CTX: u64 = 1u64 << 32;
pub const FUSE_HAS_INODE_DAX: u64 = 1u64 << 33;
pub const FUSE_CREATE_SUPP_GROUP: u64 = 1u64 << 34;
pub const FUSE_HAS_EXPIRE_ONLY: u64 = 1u64 << 35;
pub const FUSE_DIRECT_IO_ALLOW_MMAP: u64 = 1u64 << 36;
pub const FUSE_PASSTHROUGH: u64 = 1u64 << 37;
pub const FUSE_NO_EXPORT_SUPPORT: u64 = 1u64 << 38;
pub const FUSE_HAS_RESEND: u64 = 1u64 << 39;

/* Obsolete alias for FUSE_DIRECT_IO_ALLOW_MMAP */
pub const FUSE_DIRECT_IO_RELAX: u64 = FUSE_DIRECT_IO_ALLOW_MMAP;
pub const FUSE_ALLOW_IDMAP: u64 = 1 << 40;

/**
 * CUSE INIT request/reply flags
 *
 * CUSE_UNRESTRICTED_IOCTL:  use unrestricted ioctl
 */
pub const CUSE_UNRESTRICTED_IOCTL: u32 = 1 << 0;

/**
 * Release flags
 */
pub const FUSE_RELEASE_FLUSH: u32 = 1 << 0;
pub const FUSE_RELEASE_FLOCK_UNLOCK: u32 = 1 << 1;

/**
 * Getattr flags
 */
pub const FUSE_GETATTR_FH: u32 = 1 << 0;

/**
 * Lock flags
 */
pub const FUSE_LK_FLOCK: u32 = 1 << 0;

/**
 * WRITE flags
 *
 * FUSE_WRITE_CACHE: delayed write from page cache, file handle is guessed
 * FUSE_WRITE_LOCKOWNER: lock_owner field is valid
 * FUSE_WRITE_KILL_SUIDGID: kill suid and sgid bits
 */
pub const FUSE_WRITE_CACHE: u32 = 1 << 0;
pub const FUSE_WRITE_LOCKOWNER: u32 = 1 << 1;
pub const FUSE_WRITE_KILL_SUIDGID: u32 = 1 << 2;

/* Obsolete alias; this flag implies killing suid/sgid only. */
pub const FUSE_WRITE_KILL_PRIV: u32 = FUSE_WRITE_KILL_SUIDGID;

/**
 * Read flags
 */
pub const FUSE_READ_LOCKOWNER: u32 = 1 << 1;

/**
 * Ioctl flags
 *
 * FUSE_IOCTL_COMPAT: 32bit compat ioctl on 64bit machine
 * FUSE_IOCTL_UNRESTRICTED: not restricted to well-formed ioctls, retry allowed
 * FUSE_IOCTL_RETRY: retry with new iovecs
 * FUSE_IOCTL_32BIT: 32bit ioctl
 * FUSE_IOCTL_DIR: is a directory
 * FUSE_IOCTL_COMPAT_X32: x32 compat ioctl on 64bit machine (64bit time_t)
 *
 * FUSE_IOCTL_MAX_IOV: maximum of in_iovecs + out_iovecs
 */
pub const FUSE_IOCTL_COMPAT: u32 = 1 << 0;
pub const FUSE_IOCTL_UNRESTRICTED: u32 = 1 << 1;
pub const FUSE_IOCTL_RETRY: u32 = 1 << 2;
pub const FUSE_IOCTL_32BIT: u32 = 1 << 3;
pub const FUSE_IOCTL_DIR: u32 = 1 << 4;
pub const FUSE_IOCTL_COMPAT_X32: u32 = 1 << 5;

pub const FUSE_IOCTL_MAX_IOV: u32 = 256;

/**
 * Poll flags
 *
 * FUSE_POLL_SCHEDULE_NOTIFY: request poll notify
 */
pub const FUSE_POLL_SCHEDULE_NOTIFY: u32 = 1 << 0;

/**
 * Fsync flags
 *
 * FUSE_FSYNC_FDATASYNC: Sync data only, not metadata
 */
pub const FUSE_FSYNC_FDATASYNC: u32 = 1 << 0;

/**
 * fuse_attr flags
 *
 * FUSE_ATTR_SUBMOUNT: Object is a submount root
 * FUSE_ATTR_DAX: Enable DAX for this file in per inode DAX mode
 */
pub const FUSE_ATTR_SUBMOUNT: u32 = 1 << 0;
pub const FUSE_ATTR_DAX: u32 = 1 << 1;

/**
 * Open flags
 * FUSE_OPEN_KILL_SUIDGID: Kill suid and sgid if executable
 */
pub const FUSE_OPEN_KILL_SUIDGID: u32 = 1 << 0;

/**
 * setxattr flags
 * FUSE_SETXATTR_ACL_KILL_SGID: Clear SGID when system.posix_acl_access is set
 */
pub const FUSE_SETXATTR_ACL_KILL_SGID: u32 = 1 << 0;

/**
 * notify_inval_entry flags
 * FUSE_EXPIRE_ONLY
 */
pub const FUSE_EXPIRE_ONLY: u32 = 1 << 0;

/**
 * extension type
 * FUSE_MAX_NR_SECCTX: maximum value of &fuse_secctx_header.nr_secctx
 * FUSE_EXT_GROUPS: &fuse_supp_groups extension
 */
#[repr(u32)]
pub enum FuseExtType {
    /* Types 0..31 are reserved for fuse_secctx_header */
    FuseMaxNrSecctx = 31,
    FuseExtGroups = 32,
}

#[repr(u32)]
pub enum FuseOpcode {
    FuseLookup = 1,
    FuseForget = 2, /* no reply */
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
    FuseIoctl = 39,
    FusePoll = 40,
    FuseNotifyReply = 41,
    FuseBatchForget = 42,
    FuseFallocate = 43,
    FuseReaddirplus = 44,
    FuseRename2 = 45,
    FuseLseek = 46,
    FuseCopyFileRange = 47,
    FuseSetupmapping = 48,
    FuseRemovemapping = 49,
    FuseSyncfs = 50,
    FuseTmpfile = 51,
    FuseStatx = 52,

    /* CUSE specific operations */
    CuseInit = 4096,

    /* Reserved opcodes: helpful to detect structure endian-ness */
    CuseInitBswapReserved = 1048576,   /* CUSE_INIT << 8 */
    FuseInitBswapReserved = 436207616, /* FUSE_INIT << 24 */
}

#[repr(u32)]
pub enum FuseNotifyCode {
    FuseNotifyPoll = 1,
    FuseNotifyInvalInode = 2,
    FuseNotifyInvalEntry = 3,
    FuseNotifyStore = 4,
    FuseNotifyRetrieve = 5,
    FuseNotifyDelete = 6,
    FuseNotifyResend = 7,
    FuseNotifyCodeMax,
}

/* The read buffer is required to be at least 8k, but may be much larger */
pub const FUSE_MIN_READ_BUFFER: u32 = 8192;

pub const FUSE_COMPAT_ENTRY_OUT_SIZE: u32 = 120;


#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseEntryOut {
    pub nodeid: u64,      /* Inode ID */
    pub generation: u64,  /* Inode generation: nodeid:gen must be unique for the fs's lifetime */
    pub entry_valid: u64, /* Cache timeout for the name */
    pub attr_valid: u64,  /* Cache timeout for the attributes */
    pub entry_valid_nsec: u32,
    pub attr_valid_nsec: u32,
    pub attr: FuseAttr,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseForgetIn {
    pub nlookup: u64,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseForgetOne {
    pub nodeid: u64,
    pub nlookup: u64,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseBatchForgetIn {
    pub count: u32,
    pub dummy: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseGetattrIn {
    pub getattr_flags: u32,
    pub dummy: u32,
    pub fh: u64,
}

pub const FUSE_COMPAT_ATTR_OUT_SIZE: u32 = 96;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseAttrOut {
    pub attr_valid: u64, /* Cache timeout for the attributes */
    pub attr_valid_nsec: u32,
    pub dummy: u32,
    pub attr: FuseAttr,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseStatxIn {
    pub getattr_flags: u32,
    pub reserved: u32,
    pub fh: u64,
    pub sx_flags: u32,
    pub sx_mask: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseStatxOut {
    pub attr_valid: u64, /* Cache timeout for the attributes */
    pub attr_valid_nsec: u32,
    pub flags: u32,
    pub spare: [u64; 2],
    pub stat: FuseStatx,
}

pub const FUSE_COMPAT_MKNOD_IN_SIZE: u32 = 8;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseMknodIn {
    pub mode: u32,
    pub rdev: u32,
    pub umask: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseMkdirIn {
    pub mode: u32,
    pub umask: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseRenameIn {
    pub newdir: u64,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseRename2In {
    pub newdir: u64,
    pub flags: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseLinkIn {
    pub oldnodeid: u64,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseSetattrIn {
    pub valid: u32,
    pub padding: u32,
    pub fh: u64,
    pub size: u64,
    pub lock_owner: u64,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
    pub atimensec: u32,
    pub mtimensec: u32,
    pub ctimensec: u32,
    pub mode: u32,
    pub unused4: u32,
    pub uid: u32,
    pub gid: u32,
    pub unused5: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseOpenIn {
    pub flags: u32,
    pub open_flags: u32, /* FUSE_OPEN_... */
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseCreateIn {
    pub flags: u32,
    pub mode: u32,
    pub umask: u32,
    pub open_flags: u32, /* FUSE_OPEN_... */
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseOpenOut {
    pub fh: u64,
    pub open_flags: u32,
    pub backing_id: i32,
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
    pub read_flags: u32,
    pub lock_owner: u64,
    pub flags: u32,
    pub padding: u32,
}

pub const FUSE_COMPAT_WRITE_IN_SIZE: u32 = 24;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseWriteIn {
    pub fh: u64,
    pub offset: u64,
    pub size: u32,
    pub write_flags: u32,
    pub lock_owner: u64,
    pub flags: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseWriteOut {
    pub size: u32,
    pub padding: u32,
}

pub const FUSE_COMPAT_STATFS_SIZE: u32 = 48;

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

pub const FUSE_COMPAT_SETXATTR_IN_SIZE: u32 = 8;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseSetxattrIn {
    pub size: u32,
    pub flags: u32,
    pub setxattr_flags: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseGetxattrIn {
    pub size: u32,
    pub padding: u32,
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
    pub lk_flags: u32,
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
    pub flags2: u32,
    pub unused: [u32; 11],
}

pub const FUSE_COMPAT_INIT_OUT_SIZE: u32 = 8;
pub const FUSE_COMPAT_22_INIT_OUT_SIZE: u32 = 24;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseInitOut {
    pub major: u32,
    pub minor: u32,
    pub max_readahead: u32,
    pub flags: u32,
    pub max_background: u16,
    pub congestion_threshold: u16,
    pub max_write: u32,
    pub time_gran: u32,
    pub max_pages: u16,
    pub map_alignment: u16,
    pub flags2: u32,
    pub max_stack_depth: u32,
    pub unused: [u32; 6],
}

pub const CUSE_INIT_INFO_MAX: u32 = 4096;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct CuseInitIn {
    pub major: u32,
    pub minor: u32,
    pub unused: u32,
    pub flags: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct CuseInitOut {
    pub major: u32,
    pub minor: u32,
    pub unused: u32,
    pub flags: u32,
    pub max_read: u32,
    pub max_write: u32,
    pub dev_major: u32, /* chardev major */
    pub dev_minor: u32, /* chardev minor */
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

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseIoctlIn {
    pub fh: u64,
    pub flags: u32,
    pub cmd: u32,
    pub arg: u64,
    pub in_size: u32,
    pub out_size: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseIoctlIovec {
    pub base: u64,
    pub len: u64,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseIoctlOut {
    pub result: i32,
    pub flags: u32,
    pub in_iovs: u32,
    pub out_iovs: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FusePollIn {
    pub fh: u64,
    pub kh: u64,
    pub flags: u32,
    pub events: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FusePollOut {
    pub revents: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseNotifyPollWakeupOut {
    pub kh: u64,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseFallocateIn {
    pub fh: u64,
    pub offset: u64,
    pub length: u64,
    pub mode: u32,
    pub padding: u32,
}

/**
 * FUSE request unique ID flag
 *
 * Indicates whether this is a resend request. The receiver should handle this
 * request accordingly.
 */
pub const FUSE_UNIQUE_RESEND: u64 = 1 << 63;

/**
 * This value will be set by the kernel to
 * (struct fuse_in_header).{uid,gid} fields in
 * case when:
 * - fuse daemon enabled FUSE_ALLOW_IDMAP
 * - idmapping information is not available and uid/gid
 *   can not be mapped in accordance with an idmapping.
 *
 * Note: an idmapping information always available
 * for inode creation operations like:
 * FUSE_MKNOD, FUSE_SYMLINK, FUSE_MKDIR, FUSE_TMPFILE,
 * FUSE_CREATE and FUSE_RENAME2 (with RENAME_WHITEOUT).
 */
pub const FUSE_INVALID_UIDGID: u32 = u32::MAX;

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
    pub total_extlen: u16, /* length of extensions in 8byte units */
    pub padding: u16,
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
    pub type_: u32,
    pub name: [u8; 0],
}

/* Align variable length records to 64bit boundary */
pub const fn fuse_rec_align(x: usize) -> usize {
    (x + std::mem::size_of::<u64>() - 1) & !(std::mem::size_of::<u64>() - 1)
}

pub const FUSE_NAME_OFFSET: usize = std::mem::size_of::<FuseDirent>() - 0;
pub const fn fuse_dirent_align(x: usize) -> usize {
    fuse_rec_align(x)
}
pub const fn fuse_dirent_size(d: &FuseDirent) -> usize {
    fuse_dirent_align(FUSE_NAME_OFFSET + d.namelen as usize)
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseDirentplus {
    pub entry_out: FuseEntryOut,
    pub dirent: FuseDirent,
}

pub const FUSE_NAME_OFFSET_DIRENTPLUS: usize = std::mem::size_of::<FuseDirentplus>() - 0;
pub const fn fuse_direntplus_size(d: &FuseDirentplus) -> usize {
    fuse_dirent_align(FUSE_NAME_OFFSET_DIRENTPLUS + d.dirent.namelen as usize)
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseNotifyInvalInodeOut {
    pub ino: u64,
    pub off: i64,
    pub len: i64,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseNotifyInvalEntryOut {
    pub parent: u64,
    pub namelen: u32,
    pub flags: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseNotifyDeleteOut {
    pub parent: u64,
    pub child: u64,
    pub namelen: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseNotifyStoreOut {
    pub nodeid: u64,
    pub offset: u64,
    pub size: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseNotifyRetrieveOut {
    pub notify_unique: u64,
    pub nodeid: u64,
    pub offset: u64,
    pub size: u32,
    pub padding: u32,
}

/* Matches the size of fuse_write_in */
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseNotifyRetrieveIn {
    pub dummy1: u64,
    pub offset: u64,
    pub size: u32,
    pub dummy2: u32,
    pub dummy3: u64,
    pub dummy4: u64,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseBackingMap {
    pub fd: i32,
    pub flags: u32,
    pub padding: u64,
}

/* Device ioctls: */
pub const FUSE_DEV_IOC_MAGIC: u8 = 229;
pub const FUSE_DEV_IOC_CLONE: u32 = 0x8004_E500;
pub const FUSE_DEV_IOC_BACKING_OPEN: u32 = 0x4008_E501;
pub const FUSE_DEV_IOC_BACKING_CLOSE: u32 = 0x4004_E502;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseLseekIn {
    pub fh: u64,
    pub offset: u64,
    pub whence: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseLseekOut {
    pub offset: u64,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseCopyFileRangeIn {
    pub fh_in: u64,
    pub off_in: u64,
    pub nodeid_out: u64,
    pub fh_out: u64,
    pub off_out: u64,
    pub len: u64,
    pub flags: u64,
}

pub const FUSE_SETUPMAPPING_FLAG_WRITE: u64 = 1 << 0;
pub const FUSE_SETUPMAPPING_FLAG_READ: u64 = 1 << 1;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseSetupMappingIn {
    pub fh: u64,      /* An already open handle */
    pub foffset: u64, /* Offset into the file to start the mapping */
    pub len: u64,     /* Length of mapping required */
    pub flags: u64,   /* Flags, FUSE_SETUPMAPPING_FLAG_* */
    pub moffset: u64, /* Offset in Memory Window */
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseRemoveMappingIn {
    pub count: u32, /* number of fuse_removemapping_one follows */
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseRemoveMappingOne {
    pub moffset: u64, /* Offset into the dax window start the unmapping */
    pub len: u64,     /* Length of mapping required */
}

pub const FUSE_REMOVEMAPPING_MAX_ENTRY: usize =
    PAGE_SIZE / std::mem::size_of::<FuseRemoveMappingOne>();

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseSyncfsIn {
    pub padding: u64,
}

/*
 * For each security context, send fuse_secctx with size of security context
 * fuse_secctx will be followed by security context name and this in turn
 * will be followed by actual context label.
 * fuse_secctx, name, context
 */
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseSecctx {
    pub size: u32,
    pub padding: u32,
}

/*
 * Contains the information about how many fuse_secctx structures are being
 * sent and what's the total size of all security contexts (including
 * size of fuse_secctx_header).
 */
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseSecctxHeader {
    pub size: u32,
    pub nr_secctx: u32,
}

/**
 * struct fuse_ext_header - extension header
 * @size: total size of this extension including this header
 * @type: type of extension
 *
 * This is made compatible with fuse_secctx_header by using type values >
 * FUSE_MAX_NR_SECCTX
 */
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseExtHeader {
    pub size: u32,
    pub type_: u32,
}

/**
 * struct fuse_supp_groups - Supplementary group extension
 * @nr_groups: number of supplementary groups
 * @groups: flexible array of group IDs
 */
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct FuseSuppGroups {
    pub nr_groups: u32,
    pub groups: [u32; 0], /* flexible array of group IDs */
}
