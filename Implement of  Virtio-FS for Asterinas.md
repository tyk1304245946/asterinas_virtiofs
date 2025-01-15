# Implement of  Virtio-FS for Asterinas

## 设备配置空间

在我们实现的 Virtio 文件系统（virtio-fs）中，设备配置空间的设计严格遵循了 Virtio 规范，以确保与设备进行高效且兼容的通信。以下是设备配置空间的关键设计元素和实现细节。

#### 1. 设备配置结构

我们定义的 `VirtioFilesystemConfig` 结构体与 Virtio 白皮书中描述的设备配置布局一致，包含了以下字段：

- **`tag`**：一个长度为 36 字节的 UTF-8 编码字符串，用于标识文件系统的名称。如果字符串长度不足 36 字节，则会用 NUL 字节进行填充。如果编码后的字节正好填满整个字段，则该字段不以 NUL 字符结尾。
- **`num_request_queues`**：一个 32 位无符号整数，表示设备暴露的请求队列数。设备可以暴露多个 virtqueue 来提高并行处理能力，从而提升性能。
- **`notify_buf_size`**：一个 32 位无符号整数，指定通知队列中每个缓冲区的最小字节数。此字段仅在启用了 `VIRTIO_FS_F_NOTIFICATION` 特性时才有效，用于处理设备发送的 FUSE 通知消息。

#### 2. 特性标志

为了支持设备的不同功能，我们设计了 `FilesystemFeatures` 位标志结构，定义了设备的可用特性。在当前实现中，支持的特性为 `VIRTIO_FS_F_NOTIFICATION`，表示设备能够发送 FUSE 通知。该特性在设备配置中设置通知缓冲区大小时至关重要。

```rust
bitflags::bitflags! {
    pub struct FilesystemFeatures: u64 {
        const VIRTIO_FS_F_NOTIFICATION = 1 << 0;
    }
}
```

#### 3. 设备配置管理

`ConfigManager` 负责管理与 Virtio 设备配置空间的交互。通过此模块，驱动程序能够安全且高效地读取设备配置，确保所有字段的值被正确填充：

- **`new_manager`**：该方法用于初始化一个新的 `ConfigManager` 实例，负责管理设备配置内存的访问。通过 `SafePtr` 提供对配置区域的安全指针。
- **`read_config`**：该方法从设备内存中读取配置数据，包括 `tag`、`num_request_queues` 和 `notify_buf_size` 字段，确保所有配置项的正确读取和验证。

```rust
impl ConfigManager<VirtioFilesystemConfig> {
    pub(super) fn read_config(&self) -> VirtioFilesystemConfig {
        let mut fs_config = VirtioFilesystemConfig::new_uninit();

        // 读取 tag 字段
        for i in 0..fs_config.tag.len() {
            fs_config.tag[i] = self
                .read_once::<u8>(offset_of!(VirtioFilesystemConfig, tag) + i)
                .unwrap();
        }

        fs_config.num_request_queues = self
            .read_once::<u32>(offset_of!(VirtioFilesystemConfig, num_request_queues))
            .unwrap();

        fs_config.notify_buf_size = self
            .read_once::<u32>(offset_of!(VirtioFilesystemConfig, notify_buf_size))
            .unwrap();

        fs_config
    }
}
```

#### 4. 关键设计考虑

- **驱动程序合规性**：根据 Virtio 规范，驱动程序不得直接修改设备配置字段。驱动程序只能读取设备配置中的各个字段以获取设备的状态。
- **队列并行性**：设备配置中的 `num_request_queues` 字段允许驱动程序使用多个请求队列来提高性能。多个队列的使用并不会改变请求的顺序，目的是实现更高效的并行处理。
- **通知缓冲区**：如果设备启用了 `VIRTIO_FS_F_NOTIFICATION` 特性，`notify_buf_size` 字段将指定每个通知缓冲区的最小大小，这对于正确处理 FUSE 通知至关重要。

## 设备初始化

设备初始化是确保 Virtio 文件系统（virtio-fs）正常工作的关键过程。在我们的实现中，设备初始化遵循了 Virtio 规范和 FUSE 协议，确保设备在与主机交互前被正确配置。以下是设备初始化过程的详细说明。

#### 1. Virtqueue 初始化

驱动程序首先通过探测设备的 virtqueue 来进行初始化。这些 virtqueue 提供了通信的基本通道，用于处理不同类型的请求。根据设备的配置，我们创建并初始化多个 virtqueue，包括：

- **高优先级队列**：用于处理需要立即响应的请求。
- **通知队列**：如果启用了 `VIRTIO_FS_F_NOTIFICATION` 特性，则用于接收 FUSE 通知消息。
- **请求队列**：根据 `num_request_queues` 字段的值，动态分配多个请求队列，以提升系统的并发处理能力。

#### 2. 缓冲区分配

在初始化过程中，我们为每个队列分配了相应的缓冲区：

- **请求缓冲区**：这些缓冲区用于存储请求及其响应。它们通过 DMA 操作与设备内存进行数据交换。
- **通知缓冲区**：用于存储和传输 FUSE 通知消息。
- **高优先级缓冲区**：用于处理高优先级请求。

#### 3. 特性协商

设备初始化过程的第一步是协商设备特性。通过检查 `FilesystemFeatures` 位标志，我们确保只启用设备和驱动程序都支持的功能。在我们的实现中，当前支持的特性为 `VIRTIO_FS_F_NOTIFICATION`，允许设备发送 FUSE 通知。

```rust
pub(crate) fn negotiate_features(features: u64) -> u64 {
    let device_features = FilesystemFeatures::from_bits_truncate(features);
    let supported_features = FilesystemFeatures::supported_features();
    let filesystem_features = device_features & supported_features;
    debug!("features negotiated: {:?}", filesystem_features);
    filesystem_features.bits()
}
```

#### 4. FUSE 会话初始化

FUSE 会话的初始化是通过发送 `FUSE_INIT` 请求来完成的。请求由以下部分组成：

- **FuseInHeader**：包含请求的元数据。
- **FuseInitIn**：FUSE 特定的初始化数据，包括内核版本和支持的特性标志。

请求通过一个请求 virtqueue 发送到设备，并通过 DMA 机制与设备内存交换数据。初始化请求完成后，设备与驱动程序之间的 FUSE 会话即告成立。

#### 5. 中断和异步处理

初始化过程中，我们还设置了中断处理程序，用于在收到请求或通知时进行后续处理。当设备完成某个请求或发送通知时，相应的处理程序会被触发，从而实现异步的数据处理。

```rust
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
}
```

#### 6. 关键设计考虑

- **异步处理与高效通信**：通过设置多个请求队列和通知队列，驱动程序可以并行处理多个请求，并能够及时响应来自设备的通知。
- **DMA 缓冲区同步**：在发送请求之前，所有缓冲区都会与设备内存进行同步，以确保数据传输的正确性和一致性。
- **FUSE 协议遵循**：在整个初始化过程中，我们严格遵循 FUSE 协议，确保驱动程序与设备之间的交互符合协议要求。

## FUSE接口设计

### FUSE_OPENDIR

`FUSE_OPENDIR` 接口用于打开一个目录。以下是该接口的实现步骤：

1. 构建 `FuseInHeader` 和 `FuseOpenIn` 结构体，包含请求的元数据和打开目录的参数。
2. 将这些结构体转换为字节数组，并与输出缓冲区一起拼接成一个完整的请求。
3. 使用 DMA 机制将请求发送到设备，并等待设备的响应。
4. 处理设备返回的响应数据，获取打开目录的文件句柄等信息。

```rust
fn opendir(&self, nodeid: u64, flags: u32) {
    let mut request_queue = self.request_queues[0].disable_irq().lock();

    let headerin = FuseInHeader {
        len: (size_of::<FuseOpenIn>() as u32 + size_of::<FuseInHeader>() as u32),
        opcode: FuseOpcode::FuseOpendir as u32,
        unique: 0,
        nodeid: nodeid,
        uid: 0,
        gid: 0,
        pid: 0,
        total_extlen: 0,
        padding: 0,
    };

    let openin = FuseOpenIn {
        flags: flags,
        open_flags: 0,
    };

    let headerin_bytes = headerin.as_bytes();
    let openin_bytes = openin.as_bytes();
    let headerout_buffer = [0u8; size_of::<FuseOutHeader>()];
    let openout_bytes = [0u8; size_of::<FuseOpenOut>()];
    let concat_req = [
        headerin_bytes,
        openin_bytes,
        &headerout_buffer,
        &openout_bytes,
    ]
    .concat();

    let mut reader = VmReader::from(concat_req.as_slice());
    let mut writer = self.request_buffers[0].writer().unwrap();
    let len = writer.write(&mut reader);
    let len_in = size_of::<FuseOpenIn>() + size_of::<FuseInHeader>();

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
```

### FUSE_READDIR

`FUSE_READDIR` 接口用于读取目录内容。以下是该接口的实现步骤：

1. 构建 `FuseInHeader` 和 `FuseReadIn` 结构体，包含请求的元数据和读取目录的参数。
2. 将这些结构体转换为字节数组，并与输出缓冲区一起拼接成一个完整的请求。
3. 使用 DMA 机制将请求发送到设备，并等待设备的响应。
4. 处理设备返回的响应数据，解析目录项并输出目录内容。

```rust
fn readdir(&self, nodeid: u64, fh: u64, offset: u64, size: u32) {
    let mut request_queue = self.request_queues[0].disable_irq().lock();

    let headerin = FuseInHeader {
        len: (size_of::<FuseReadIn>() as u32 + size_of::<FuseInHeader>() as u32),
        opcode: FuseOpcode::FuseReaddir as u32,
        unique: 0,
        nodeid: nodeid,
        uid: 0,
        gid: 0,
        pid: 0,
        total_extlen: 0,
        padding: 0,
    };

    let readin = FuseReadIn {
        fh: fh,
        offset: offset,
        size: size,
        read_flags: 0,
        lock_owner: 0,
        flags: 0,
        padding: 0,
    };

    let headerin_bytes = headerin.as_bytes();
    let readin_bytes = readin.as_bytes();
    let headerout_buffer = [0u8; size_of::<FuseOutHeader>()];
    let readout_bytes = [0u8; 1024];
    let concat_req = [
        headerin_bytes,
        &readin_bytes,
        &headerout_buffer,
        &readout_bytes,
    ]
    .concat();

    let mut reader = VmReader::from(concat_req.as_slice());
    let mut writer = self.request_buffers[0].writer().unwrap();
    let len = writer.write(&mut reader);
    let len_in = size_of::<FuseReadIn>() + size_of::<FuseInHeader>();

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
```
