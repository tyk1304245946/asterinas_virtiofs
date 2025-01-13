// SPDX-License-Identifier: MPL-2.0

use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::fmt::Debug;

use log::debug;
use ostd::{
    early_print, early_println,
    mm::{DmaDirection, DmaStream, DmaStreamSlice, FrameAllocOptions, VmReader},
    sync::SpinLock,
    trap::TrapFrame,
    Pod,
};

use super::{
    config::{FilesystemFeatures, VirtioFilesystemConfig},
    fuse::*,
    request::AnyFuseDevice,
};
use crate::{
    device::VirtioDeviceError,
    queue::VirtQueue,
    transport::{ConfigManager, VirtioTransport},
};

pub struct FilesystemDevice {
    config_manager: ConfigManager<VirtioFilesystemConfig>,
    transport: SpinLock<Box<dyn VirtioTransport>>,

    hiprio_queue: SpinLock<VirtQueue>,
    request_queues: Vec<SpinLock<VirtQueue>>,
    notify_queue: SpinLock<VirtQueue>,

    hiprio_buffer: DmaStream,
    request_buffers: Vec<DmaStream>,
    notify_buffer: DmaStream,
    // callbacks: RwLock<Vec<&'static FilesystemCallback>, LocalIrqDisabled>,
}

impl AnyFuseDevice for FilesystemDevice {
    fn init(&self) {
        let mut request_queue = self.request_queues[0].disable_irq().lock();
        // let request_buffer = device.request_buffers[0].clone();
        let headerin = FuseInHeader {
            len: (size_of::<FuseInitIn>() as u32 + size_of::<FuseInHeader>() as u32),
            opcode: FuseOpcode::FuseInit as u32,
            unique: 0,
            nodeid: 0,
            uid: 0,
            gid: 0,
            pid: 0,
            total_extlen: 0,
            padding: 0,
        };
        let initin = FuseInitIn {
            major: FUSE_KERNEL_VERSION,
            minor: FUSE_KERNEL_MINOR_VERSION,
            max_readahead: 0,
            flags: FuseInitFlags::FUSE_INIT_EXT.bits() as u32,
            flags2: 0,
            unused: [0u32; 11],
        };
        let headerin_bytes = headerin.as_bytes();
        let initin_bytes = initin.as_bytes();
        let headerout_buffer = [0u8; size_of::<FuseOutHeader>()];
        let initout_bytes = [0u8; 256];
        let concat_req = [
            headerin_bytes,
            initin_bytes,
            &headerout_buffer,
            &initout_bytes,
        ]
        .concat();
        // Send msg
        let mut reader = VmReader::from(concat_req.as_slice());
        let mut writer = self.request_buffers[0].writer().unwrap();
        let len = writer.write(&mut reader);
        let len_in = size_of::<FuseInitIn>() + size_of::<FuseInHeader>();
        self.request_buffers[0].sync(0..len).unwrap();
        let slice_in = DmaStreamSlice::new(&self.request_buffers[0], 0, len_in);
        let slice_out = DmaStreamSlice::new(&self.request_buffers[0], len_in, len);
        request_queue
            .add_dma_buf(&[&slice_in], &[&slice_out])
            .unwrap();
        if request_queue.should_notify() {
            request_queue.notify();
        }
    }
}

impl Debug for FilesystemDevice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FilesystemDevice")
            .field("config_manager", &self.config_manager)
            .field("transport", &self.transport)
            .field("hiprio_queue", &self.hiprio_queue)
            .field("request_queues", &self.request_queues)
            .field("notify_queue", &self.notify_queue)
            .field("hiprio_buffer", &self.hiprio_buffer)
            .field("request_buffers", &self.request_buffers)
            .field("notify_buffer", &self.notify_buffer)
            .finish()
    }
}

impl FilesystemDevice {
    /// Negotiate features for the device specified bits 0~23
    pub(crate) fn negotiate_features(features: u64) -> u64 {
        let device_features = FilesystemFeatures::from_bits_truncate(features);
        let supported_features = FilesystemFeatures::supported_features();
        let filesystem_features = device_features & supported_features;
        debug!("features negotiated: {:?}", filesystem_features);
        filesystem_features.bits()
    }

    pub fn init(mut transport: Box<dyn VirtioTransport>) -> Result<(), VirtioDeviceError> {
        let config_manager = VirtioFilesystemConfig::new_manager(&*transport);

        let fs_config = config_manager.read_config();
        early_println!("virtio_fs_config = {:?}", fs_config);
        debug!("virtio_fs_config = {:?}", fs_config);

        const HIPRIO_QUEUE_INDEX: u16 = 0;
        const NOTIFICATION_QUEUE_INDEX: u16 = 1;
        const REQUEST_QUEUE_BASE_INDEX: u16 = 1;

        let hiprio_queue =
            SpinLock::new(VirtQueue::new(HIPRIO_QUEUE_INDEX, 2, transport.as_mut()).unwrap());
        let notify_queue =
            SpinLock::new(VirtQueue::new(NOTIFICATION_QUEUE_INDEX, 2, transport.as_mut()).unwrap());
        let mut request_queues = Vec::new();
        for i in 0..fs_config.num_request_queues {
            request_queues.push(SpinLock::new(
                VirtQueue::new(REQUEST_QUEUE_BASE_INDEX + (i as u16), 2, transport.as_mut())
                    .unwrap(),
            ))
        }

        let hiprio_buffer = {
            let segment = FrameAllocOptions::new().alloc_segment(1).unwrap();
            DmaStream::map(segment.into(), DmaDirection::ToDevice, false).unwrap()
        };
        let notify_buffer = {
            let segment = FrameAllocOptions::new().alloc_segment(1).unwrap();
            DmaStream::map(segment.into(), DmaDirection::ToDevice, false).unwrap()
        };
        let mut request_buffers = Vec::new();
        for _ in 0..fs_config.num_request_queues {
            let request_buffer = {
                let vm_segment = FrameAllocOptions::new().alloc_segment(1).unwrap();
                DmaStream::map(vm_segment.into(), DmaDirection::Bidirectional, false).unwrap()
            };
            request_buffers.push(request_buffer);
        }

        let device = Arc::new(Self {
            config_manager: config_manager,
            transport: SpinLock::new(transport),
            hiprio_queue: hiprio_queue,
            notify_queue: notify_queue,
            request_queues: request_queues,
            hiprio_buffer: hiprio_buffer,
            notify_buffer: notify_buffer,
            request_buffers: request_buffers,
        });

        let handle_reqest = {
            let device = device.clone();
            move |_: &TrapFrame| device.handle_recv_irq()
        };

        let config_space_change = |_: &TrapFrame| early_print!("Config Changed\n");
        let mut transport = device.transport.disable_irq().lock();
        transport
            .register_queue_callback(REQUEST_QUEUE_BASE_INDEX + 0, Box::new(handle_reqest), false)
            .unwrap();
        transport
            .register_cfg_callback(Box::new(config_space_change))
            .unwrap();
        transport.finish_init();
        drop(transport);

        device.init();

        // test_device(device);

        Ok(())
    }

    fn handle_recv_irq(&self) {
        let mut request_queue = self.request_queues[0].disable_irq().lock();
        let Ok((_, len)) = request_queue.pop_used() else {
            return;
        };
        self.request_buffers[0].sync(0..len as usize).unwrap();

        let mut reader = self.request_buffers[0].reader().unwrap();
        let headerin = reader.read_val::<FuseInHeader>().unwrap();
        let datain = reader.read_val::<FuseInitIn>().unwrap();
        let headerout = reader.read_val::<FuseOutHeader>().unwrap();
        let dataout = reader.read_val::<FuseInitOut>().unwrap();

        match FuseOpcode::try_from(headerin.opcode).unwrap() {
            FuseOpcode::FuseInit => {
                let dataout = reader.read_val::<FuseInitOut>().unwrap();
                early_print!("Received Init Msg\n");
                early_print!("major:{:?}\n", dataout.major);
                early_print!("minor:{:?}\n", dataout.minor);
                early_print!("flags:{:?}\n", dataout.flags);
            }
            _ => {}
        }

        // early_print!("Received Msg:\n");
        // early_print!("headerin:{:?}\n", headerin);
        // early_print!("datain:{:?}\n", datain);
        // early_print!("headerout:{:?}\n", headerout);
        // early_print!("dataout:{:?}\n", dataout);
    }
}

// fn test_device(device: Arc<FilesystemDevice>) {
//     let mut request_queue = device.request_queues[0].disable_irq().lock();
//     // let request_buffer = device.request_buffers[0].clone();
//     let headerin = FuseInHeader {
//         len: (size_of::<FuseInitIn>() as u32 + size_of::<FuseInHeader>() as u32),
//         opcode: FuseOpcode::FuseInit as u32,
//         unique: 0,
//         nodeid: 0,
//         uid: 0,
//         gid: 0,
//         pid: 0,
//         total_extlen: 0,
//         padding: 0,
//     };
//     let initin = FuseInitIn {
//         major: FUSE_KERNEL_VERSION,
//         minor: FUSE_KERNEL_MINOR_VERSION,
//         max_readahead: 0,
//         flags: FuseInitFlags::FUSE_INIT_EXT.bits() as u32,
//         flags2: 0,
//         unused: [0u32; 11],
//     };
//     let headerin_bytes = headerin.as_bytes();
//     let initin_bytes = initin.as_bytes();
//     let headerout_buffer = [0u8; size_of::<FuseOutHeader>()];
//     let initout_bytes = [0u8; 256];
//     let concat_req = [
//         headerin_bytes,
//         initin_bytes,
//         &headerout_buffer,
//         &initout_bytes,
//     ]
//     .concat();
//     // Send msg
//     let mut reader = VmReader::from(concat_req.as_slice());
//     let mut writer = device.request_buffers[0].writer().unwrap();
//     let len = writer.write(&mut reader);
//     let len_in = size_of::<FuseInitIn>() + size_of::<FuseInHeader>();
//     device.request_buffers[0].sync(0..len).unwrap();
//     let slice_in = DmaStreamSlice::new(&device.request_buffers[0], 0, len_in);
//     let slice_out = DmaStreamSlice::new(&device.request_buffers[0], len_in, len);
//     request_queue
//         .add_dma_buf(&[&slice_in], &[&slice_out])
//         .unwrap();
//     if request_queue.should_notify() {
//         request_queue.notify();
//     }
// }
