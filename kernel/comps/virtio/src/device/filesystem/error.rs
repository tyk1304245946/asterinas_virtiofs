// SPDX-License-Identifier: MPL-2.0

use core::{fmt, result};

use crate::queue::QueueError;

/// The error type of VirtIO socket driver.
#[derive(Debug)]
pub enum FilesystemError {
    /// The given buffer is shorter than expected.
    BufferTooShort,
    /// The given buffer for output is shorter than expected.
    OutputBufferTooShort(usize),
    /// The given buffer has exceeded the maximum buffer size.
    BufferTooLong(usize, usize),
    /// Unknown operation.
    UnknownOperation(u16),
    /// Invalid operation,
    InvalidOperation,
    /// Invalid number.
    InvalidNumber,
    /// Unexpected data in packet.
    UnexpectedDataInPacket,
    /// Peer has insufficient buffer space, try again later.
    InsufficientBufferSpaceInPeer,
    /// Recycled a wrong buffer.
    RecycledWrongBuffer,
    /// Queue Error
    QueueError(QueueError),
}

impl From<QueueError> for FilesystemError {
    fn from(value: QueueError) -> Self {
        Self::QueueError(value)
    }
}

impl From<int_to_c_enum::TryFromIntError> for FilesystemError {
    fn from(_e: int_to_c_enum::TryFromIntError) -> Self {
        Self::InvalidNumber
    }
}

impl fmt::Display for FilesystemError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::BufferTooShort => write!(f, "The given buffer is shorter than expected"),
            Self::BufferTooLong(actual, max) => {
                write!(f, "The given buffer length '{actual}' has exceeded the maximum allowed buffer length '{max}'")
            }
            Self::OutputBufferTooShort(expected) => {
                write!(f, "The given output buffer is too short. '{expected}' bytes is needed for the output buffer.")
            }
            Self::UnknownOperation(op) => {
                write!(f, "The operation code '{op}' is unknown")
            }
            Self::InvalidOperation => write!(f, "Invalid operation"),
            Self::InvalidNumber => write!(f, "Invalid number"),
            Self::UnexpectedDataInPacket => write!(f, "No data is expected in the packet"),
            Self::InsufficientBufferSpaceInPeer => {
                write!(f, "Peer has insufficient buffer space, try again later")
            }
            Self::RecycledWrongBuffer => write!(f, "Recycled a wrong buffer"),
            Self::QueueError(_) => write!(f, "Error encountered out of vsock itself!"),
        }
    }
}

pub type Result<T> = result::Result<T, FilesystemError>;
