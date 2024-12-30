// SPDX-License-Identifier: MPL-2.0

use alloc::boxed::Box;
use core::fmt::Debug;

use log::debug;
use ostd::{
    early_println,
    mm::{DmaDirection, DmaStream, FrameAllocOptions},
    sync::SpinLock,
};

use super::config::VirtioFilesystemConfig;
use crate::{
    device::{filesystem::config::FilesystemFeatures, VirtioDeviceError},
    queue::VirtQueue,
    transport::{ConfigManager, VirtioTransport},
};

pub struct FilesystemDevice {
    config_manager: ConfigManager<VirtioFilesystemConfig>,
    transport: SpinLock<Box<dyn VirtioTransport>>,
    receive_queue: SpinLock<VirtQueue>,
    transmit_queue: SpinLock<VirtQueue>,
    send_buffer: DmaStream,
    receive_buffer: DmaStream,
    // callbacks: RwLock<Vec<&'static FilesystemCallback>, LocalIrqDisabled>,
}

impl Debug for FilesystemDevice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FilesystemDevice")
            .field("config_manager", &self.config_manager)
            .field("transport", &self.transport)
            .field("receive_queue", &self.receive_queue)
            .field("transmit_queue", &self.transmit_queue)
            .field("send_buffer", &self.send_buffer)
            .field("receive_buffer", &self.receive_buffer)
            .finish()
    }
}

impl FilesystemDevice {
    pub fn negotiate_features(features: u64) -> u64 {
        let features = FilesystemFeatures::from_bits_truncate(features);
        features.bits()
    }

    pub fn init(mut transport: Box<dyn VirtioTransport>) -> Result<(), VirtioDeviceError> {
        let config_manager = VirtioFilesystemConfig::new_manager(&*transport);
        early_println!("virtio_fs_config = {:?}", config_manager.read_config());
        debug!("virtio_fs_config = {:?}", config_manager.read_config());

        const RECV0_QUEUE_INDEX: u16 = 0;
        const TRANSMIT0_QUEUE_INDEX: u16 = 1;
        let receive_queue =
            SpinLock::new(VirtQueue::new(RECV0_QUEUE_INDEX, 2, transport.as_mut()).unwrap());
        let transmit_queue =
            SpinLock::new(VirtQueue::new(TRANSMIT0_QUEUE_INDEX, 2, transport.as_mut()).unwrap());

        let send_buffer = {
            let segment = FrameAllocOptions::new().alloc_segment(1).unwrap();
            DmaStream::map(segment.into(), DmaDirection::ToDevice, false).unwrap()
        };

        let receive_buffer = {
            let segment = FrameAllocOptions::new().alloc_segment(1).unwrap();
            DmaStream::map(segment.into(), DmaDirection::FromDevice, false).unwrap()
        };

        let device = FilesystemDevice {
            config_manager,
            transport: SpinLock::new(transport),
            receive_queue,
            transmit_queue,
            send_buffer,
            receive_buffer,
        };

        // device.activate_receive_buffer(&mut device.receive_queue.disable_irq().lock());

        Ok(())
    }
}
