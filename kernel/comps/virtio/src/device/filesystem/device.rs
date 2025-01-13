// SPDX-License-Identifier: MPL-2.0

use alloc::{boxed::Box, string::ToString, sync::Arc, vec::Vec};
use core::{fmt::Debug, iter::Fuse};

use aster_filesystem::{RxBuffer, TxBuffer};
use log::debug;
use ostd::{
    early_println,
    mm::{DmaDirection, DmaStream, DmaStreamSlice, FrameAllocOptions, VmReader},
    sync::{RwLock, SpinLock},
    trap::TrapFrame,
};

use super::{config::VirtioFilesystemConfig, error::FilesystemError, fuse::*};
use crate::{
    device::{
        filesystem::buffer::{RX_BUFFER_POOL, TX_BUFFER_POOL},
        VirtioDeviceError,
    },
    queue::{QueueError, VirtQueue},
    transport::{ConfigManager, VirtioTransport},
};

const QUEUE_SIZE: u16 = 64;
const QUEUE_RECV: u16 = 0;
const QUEUE_SEND: u16 = 1;
const QUEUE_EVENT: u16 = 2;

pub struct FilesystemDevice {
    config_manager: ConfigManager<VirtioFilesystemConfig>,

    recv_queue: VirtQueue,
    send_queue: VirtQueue,
    event_queue: VirtQueue,

    send_buffer: DmaStream,
    receive_buffer: DmaStream,

    transport: Box<dyn VirtioTransport>,
    // callbacks: RwLock<Vec<&'static FilesystemCallback>, LocalIrqDisabled>,
}

impl FilesystemDevice {
    // Crate a new filesystem device
    pub fn init(mut transport: Box<dyn VirtioTransport>) -> Result<(), VirtioDeviceError> {
        let config_manager = VirtioFilesystemConfig::new_manager(&*transport);
        early_println!("virtio_fs_config = {:?}", config_manager.read_config());
        debug!("virtio_fs_config = {:?}", config_manager.read_config());

        const RECV0_QUEUE_INDEX: u16 = 0;
        const TRANSMIT0_QUEUE_INDEX: u16 = 1;

        let recv_queue = VirtQueue::new(QUEUE_RECV, QUEUE_SIZE, transport.as_mut())
            .expect("creating recv queue fails");
        let send_queue = VirtQueue::new(QUEUE_SEND, QUEUE_SIZE, transport.as_mut())
            .expect("creating send queue fails");
        let event_queue = VirtQueue::new(QUEUE_EVENT, QUEUE_SIZE, transport.as_mut())
            .expect("creating event queue fails");

        let send_buffer: DmaStream = {
            let segment = FrameAllocOptions::new().alloc_segment(1).unwrap();
            DmaStream::map(segment.into(), DmaDirection::ToDevice, false).unwrap()
        };

        let receive_buffer = {
            let segment = FrameAllocOptions::new().alloc_segment(1).unwrap();
            DmaStream::map(segment.into(), DmaDirection::FromDevice, false).unwrap()
        };

        let mut device: FilesystemDevice = Self {
            config_manager,
            recv_queue,
            send_queue,
            event_queue,
            send_buffer,
            receive_buffer,
            transport,
            // callbacks: RwLock::new(Vec::new()),
        };

        // Interrupt handler if filesystem device config space changes
        fn config_space_change(_: &TrapFrame) {
            debug!("filesystem device config space change");
        }

        // Interrupt handler if filesystem device receives some request.
        fn handle_virtofs_event(_: &TrapFrame) {
            // handle_recv_irq(super::DEVICE_NAME);
        }

        // Register irq callbacks
        device
            .transport
            .register_cfg_callback(Box::new(config_space_change))
            .unwrap();
        device
            .transport
            .register_queue_callback(QUEUE_RECV, Box::new(handle_virtofs_event), false)
            .unwrap();

        device.transport.finish_init();

        // register_device(
        //     super::DEVICE_NAME.to_string(),
        //     Arc::new(SpinLock::new(device)),
        // );

        Ok(())
    }

    fn send_request_to_tx_queue(
        &self,
        header: &FuseInHeader,
        buffer: &[u8],
    ) -> Result<(), FilesystemError> {
        // debug!("Sent request {:?}. Op {:?}", header, header.op());
        debug!("buffer in send_request_to_tx_queue: {:?}", buffer);
        let tx_buffer: TxBuffer = {
            let pool = TX_BUFFER_POOL.get().unwrap();
            TxBuffer::new(header, buffer, pool)
        };

        let token = self.send_queue.add_dma_buf(&[&tx_buffer], &[])?;

        if self.send_queue.should_notify() {
            self.send_queue.notify();
        }

        // Wait until the buffer is used
        while !self.send_queue.can_pop() {
            spin_loop();
        }

        // Pop out the buffer, so we can reuse the send queue further
        let (pop_token, _) = self.send_queue.pop_used()?;
        debug_assert!(pop_token == token);
        if pop_token != token {
            return Err(FilesystemError::QueueError(QueueError::WrongToken));
        }
        debug!("send request succeeds");
        Ok(())
    }

    // fn check_peer_buffer_is_sufficient(
    //     &mut self,
    //     connection_info: &mut ConnectionInfo,
    //     buffer_len: usize,
    // ) -> Result<(), FilesystemError> {
    //     debug!("connection info {:?}", connection_info);
    //     debug!(
    //         "peer free from peer: {:?}, buffer len : {:?}",
    //         connection_info.peer_free(),
    //         buffer_len
    //     );
    //     if connection_info.peer_free() as usize >= buffer_len {
    //         Ok(())
    //     } else {
    //         // Request an update of the cached peer credit, if we haven't already done so, and tell
    //         // the caller to try again later.
    //         if !connection_info.has_pending_credit_request {
    //             self.credit_request(connection_info)?;
    //             connection_info.has_pending_credit_request = true;
    //             //TODO check if the update needed
    //         }
    //         Err(FilesystemError::InsufficientBufferSpaceInPeer)
    //     }
    // }

    /// Sends the buffer to the destination.
    pub fn send(
        &mut self,
        buffer: &[u8],
        // connection_info: &mut ConnectionInfo,
    ) -> Result<(), FilesystemError> {
        // self.check_peer_buffer_is_sufficient(connection_info, buffer.len())?;

        let len = buffer.len() as u32;
        let header = FuseInHeader {
            // op: VirtioVsockOp::Rw as u16,
            len,
            // ..connection_info.new_header(self.guest_cid)
        };
        // connection_info.tx_cnt += len;
        self.send_request_to_tx_queue(&header, buffer)
    }

    /// Receive bytes from peer, returns the header
    pub fn receive(
        &mut self,
        // connection_info: &mut ConnectionInfo,
    ) -> Result<RxBuffer, FilesystemError> {
        let (token, len) = self.recv_queue.pop_used()?;
        debug!(
            "receive request in rx_queue: token = {}, len = {}",
            token, len
        );
        let mut rx_buffer = self
            .rx_buffers
            .remove(token as usize)
            .ok_or(QueueError::WrongToken)?;
        rx_buffer.set_request_len(len as usize);

        let rx_pool = RX_BUFFER_POOL.get().unwrap();
        let new_rx_buffer = RxBuffer::new(size_of::<VirtioVsockHdr>(), rx_pool);
        self.add_rx_buffer(new_rx_buffer, token)?;

        Ok(rx_buffer)
    }

    /// Polls the RX virtqueue for the next event, and calls the given handler function to handle it.
    pub fn poll(
        &mut self,
        handler: impl FnOnce(VsockEvent, &[u8]) -> Result<Option<VsockEvent>, FilesystemError>,
    ) -> Result<Option<VsockEvent>, FilesystemError> {
        // Return None if there is no pending request.
        if !self.recv_queue.can_pop() {
            return Ok(None);
        }
        let rx_buffer = self.receive()?;

        let mut buf_reader = rx_buffer.buf();
        let mut temp_buffer = vec![0u8; buf_reader.remain()];
        buf_reader.read(&mut VmWriter::from(&mut temp_buffer as &mut [u8]));

        let (header, payload) = read_header_and_body(&temp_buffer)?;
        // The length written should be equal to len(header)+len(request)
        debug!("Received request {:?}. Op {:?}", header, header.op());
        debug!("body is {:?}", payload);
        VsockEvent::from_header(&header).and_then(|event| handler(event, payload))
    }

    /// Add a used rx buffer to recv queue,@index is only to check the correctness
    fn add_rx_buffer(&mut self, rx_buffer: RxBuffer, index: u16) -> Result<(), FilesystemError> {
        let token = self.recv_queue.add_dma_buf(&[], &[&rx_buffer])?;
        assert_eq!(index, token);
        assert!(self.rx_buffers.put_at(token as usize, rx_buffer).is_none());
        if self.recv_queue.should_notify() {
            self.recv_queue.notify();
        }
        Ok(())
    }

    // Neogotiate features for the filesystem device
    pub fn negotiate_features(features: u64) -> u64 {
        let features = FilesystemFeatures::from_bits_truncate(features);
        features.bits()
    }
}

impl Debug for FilesystemDevice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FilesystemDevice")
            .field("config_manager", &self.config_manager)
            .field("recv_queue", &self.recv_queue)
            .field("send_queue", &self.send_queue)
            .field("event_queue", &self.event_queue)
            .field("send_buffer", &self.send_buffer)
            .field("receive_buffer", &self.receive_buffer)
            .field("transport", &self.transport)
            // .field("callbacks", &self.callbacks)
            .finish()
    }
}

fn read_header_and_body(buffer: &[u8]) -> Result<(FuseInHeader, &[u8]), FilesystemError> {
    // Shouldn't panic, because we know `RX_BUFFER_SIZE > size_of::<VirtioVsockHdr>()`.
    let header = VirtioVsockHdr::from_bytes(&buffer[..VIRTIO_VSOCK_HDR_LEN]);
    let body_length = header.len() as usize;

    // This could fail if the device returns an unreasonably long body length.
    let data_end = VIRTIO_VSOCK_HDR_LEN
        .checked_add(body_length)
        .ok_or(FilesystemError::InvalidNumber)?;
    // This could fail if the device returns a body length longer than the buffer we gave it.
    let data = buffer
        .get(VIRTIO_VSOCK_HDR_LEN..data_end)
        .ok_or(FilesystemError::BufferTooShort)?;
    Ok((header, data))
}
