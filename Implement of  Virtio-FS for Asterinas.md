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

1. **输入**：
   - `FuseInHeader`：包含请求的元数据，包括请求长度、操作码、唯一标识符、节点 ID、用户 ID、组 ID、进程 ID 等。
   - `FuseOpenIn`：包含打开目录的参数，包括标志位和打开标志。

2. **输出**：
   - `FuseOutHeader`：包含响应的元数据，包括响应长度、错误码、唯一标识符等。
   - `FuseOpenOut`：包含打开目录的响应数据，包括文件句柄、打开标志和后备 ID。

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

1. **输入**：
   - `FuseInHeader`：包含请求的元数据，包括请求长度、操作码、唯一标识符、节点 ID、用户 ID、组 ID、进程 ID 等。
   - `FuseReadIn`：包含读取目录的参数，包括文件句柄、偏移量、读取大小、读取标志、锁所有者和标志。

2. **输出**：
   - `FuseOutHeader`：包含响应的元数据，包括响应长度、错误码、唯一标识符等。
   - `FuseDirent` 和 `FuseDirentWithName`：包含目录项的元数据和名称。

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

### FUSE_OPEN

`FUSE_OPEN` 接口用于打开一个文件。以下是该接口的实现步骤：

1. **输入**：
   - `FuseInHeader`：包含请求的元数据，包括请求长度、操作码、唯一标识符、节点 ID、用户 ID、组 ID、进程 ID 等。
   - `FuseOpenIn`：包含打开文件的参数，包括标志位和打开标志。

2. **输出**：
   - `FuseOutHeader`：包含响应的元数据，包括响应长度、错误码、唯一标识符等。
   - `FuseOpenOut`：包含打开文件的响应数据，包括文件句柄、打开标志和后备 ID。

```rust
fn open(&self, nodeid: u64, flags: u32) {
    let mut request_queue = self.request_queues[0].disable_irq().lock();

    let headerin = FuseInHeader {
        len: (size_of::<FuseOpenIn>() as u32 + size_of::<FuseInHeader>() as u32),
        opcode: FuseOpcode::FuseOpen as u32,
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

### FUSE_READ

`FUSE_READ` 接口用于读取文件内容。以下是该接口的实现步骤：

1. **输入**：
   - `FuseInHeader`：包含请求的元数据，包括请求长度、操作码、唯一标识符、节点 ID、用户 ID、组 ID、进程 ID 等。
   - `FuseReadIn`：包含读取文件的参数，包括文件句柄、偏移量、读取大小、读取标志、锁所有者和标志。

2. **输出**：
   - `FuseOutHeader`：包含响应的元数据，包括响应长度、错误码、唯一标识符等。
   - 文件内容：包含读取的文件数据。

```rust
fn read(&self, nodeid: u64, fh: u64, offset: u64, size: u32) {
    let mut request_queue = self.request_queues[0].disable_irq().lock();

    let headerin = FuseInHeader {
        len: (size_of::<FuseReadIn>() as u32 + size_of::<FuseInHeader>() as u32),
        opcode: FuseOpcode::FuseRead as u32,
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

### FUSE_FLUSH

`FUSE_FLUSH` 接口用于刷新文件的缓冲区。以下是该接口的实现步骤：

1. **输入**：
   - `FuseInHeader`：包含请求的元数据，包括请求长度、操作码、唯一标识符、节点 ID、用户 ID、组 ID、进程 ID 等。
   - `FuseFlushIn`：包含刷新文件的参数，包括文件句柄和锁所有者。

2. **输出**：
   - `FuseOutHeader`：包含响应的元数据，包括响应长度、错误码、唯一标识符等。

```rust
fn flush(&self, nodeid: u64, fh: u64, lock_owner: u64) {
    let mut request_queue = self.request_queues[0].disable_irq().lock();

    let headerin = FuseInHeader {
        len: (size_of::<FuseFlushIn>() as u32 + size_of::<FuseInHeader>() as u32),
        opcode: FuseOpcode::FuseFlush as u32,
        unique: 0,
        nodeid: nodeid,
        uid: 0,
        gid: 0,
        pid: 0,
        total_extlen: 0,
        padding: 0,
    };

    let flushin = FuseFlushIn {
        fh: fh,
        lock_owner: lock_owner,
        padding: 0,
        unused: 0,
    };

    let headerin_bytes = headerin.as_bytes();
    let flushin_bytes = flushin.as_bytes();
    let headerout_buffer = [0u8; size_of::<FuseOutHeader>()];
    let concat_req = [
        headerin_bytes,
        flushin_bytes,
        &headerout_buffer,
    ]
    .concat();

    let mut reader = VmReader::from(concat_req.as_slice());
    let mut writer = self.request_buffers[0].writer().unwrap();
    let len = writer.write(&mut reader);
    let len_in = size_of::<FuseFlushIn>() + size_of::<FuseInHeader>();

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

### FUSE_RELEASEDIR

`FUSE_RELEASEDIR` 接口用于关闭一个打开的目录。以下是该接口的实现步骤：

1. **输入**：
   - `FuseInHeader`：包含请求的元数据，包括请求长度、操作码、唯一标识符、节点 ID、用户 ID、组 ID、进程 ID 等。
   - `FuseReleaseIn`：包含关闭目录的参数，包括文件句柄和标志位。

2. **输出**：
   - `FuseOutHeader`：包含响应的元数据，包括响应长度、错误码、唯一标识符等。

```rust
fn releasedir(&self, nodeid: u64, fh: u64, flags: u32) {
    let mut request_queue = self.request_queues[0].disable_irq().lock();

    let headerin = FuseInHeader {
        len: (size_of::<FuseReleaseIn>() as u32 + size_of::<FuseInHeader>() as u32),
        opcode: FuseOpcode::FuseReleasedir as u32,
        unique: 0,
        nodeid: nodeid,
        uid: 0,
        gid: 0,
        pid: 0,
        total_extlen: 0,
        padding: 0,
    };

    let releasein = FuseReleaseIn {
        fh: fh,
        flags: flags,
        lock_owner: 0,
        padding: 0,
    };

    let headerin_bytes = headerin.as_bytes();
    let releasein_bytes = releasein.as_bytes();
    let headerout_buffer = [0u8; size_of::<FuseOutHeader>()];
    let concat_req = [
        headerin_bytes,
        releasein_bytes,
        &headerout_buffer,
    ]
    .concat();

    let mut reader = VmReader::from(concat_req.as_slice());
    let mut writer = self.request_buffers[0].writer().unwrap();
    let len = writer.write(&mut reader);
    let len_in = size_of::<FuseReleaseIn>() + size_of::<FuseInHeader>();

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

### FUSE_GETATTR

`FUSE_GETATTR` 接口用于获取文件或目录的属性。以下是该接口的实现步骤：

1. **输入**：
   - `FuseInHeader`：包含请求的元数据，包括请求长度、操作码、唯一标识符、节点 ID、用户 ID、组 ID、进程 ID 等。
   - `FuseGetattrIn`：包含获取属性的参数，包括文件句柄、标志位和占位符。

2. **输出**：
   - `FuseOutHeader`：包含响应的元数据，包括响应长度、错误码、唯一标识符等。
   - `FuseAttrOut`：包含文件或目录的属性数据。

```rust
fn getattr(&self, nodeid: u64, fh: u64, flags: u32, dummy: u32) {
    let mut request_queue = self.request_queues[0].disable_irq().lock();

    let headerin = FuseInHeader {
        len: (size_of::<FuseGetattrIn>() as u32 + size_of::<FuseInHeader>() as u32),
        opcode: FuseOpcode::FuseGetattr as u32,
        unique: 0,
        nodeid: nodeid,
        uid: 0,
        gid: 0,
        pid: 0,
        total_extlen: 0,
        padding: 0,
    };

    let getattrin = FuseGetattrIn {
        fh: fh,
        flags: flags,
        dummy: dummy,
    };

    let headerin_bytes = headerin.as_bytes();
    let getattrin_bytes = getattrin.as_bytes();
    let headerout_buffer = [0u8; size_of::<FuseOutHeader>()];
    let getattrout_bytes = [0u8; size_of::<FuseAttrOut>()];
    let concat_req = [
        headerin_bytes,
        getattrin_bytes,
        &headerout_buffer,
        &getattrout_bytes,
    ]
    .concat();

    let mut reader = VmReader::from(concat_req.as_slice());
    let mut writer = self.request_buffers[0].writer().unwrap();
    let len = writer.write(&mut reader);
    let len_in = size_of::<FuseGetattrIn>() + size_of::<FuseInHeader>();

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

### FUSE_LOOKUP

`FUSE_LOOKUP` 接口用于查找目录中的文件或子目录。以下是该接口的实现步骤：

1. **输入**：
   - `FuseInHeader`：包含请求的元数据，包括请求长度、操作码、唯一标识符、节点 ID、用户 ID、组 ID、进程 ID 等。
   - 文件名：包含要查找的文件或子目录的名称。

2. **输出**：
   - `FuseOutHeader`：包含响应的元数据，包括响应长度、错误码、唯一标识符等。
   - `FuseEntryOut`：包含查找到的文件或子目录的元数据。

```rust
fn lookup(&self, nodeid: u64, name: Vec<u8>) {
    let mut request_queue = self.request_queues[0].disable_irq().lock();

    // 添加终止符 '\0' 到名称
    let mut name = name;
    name.push(0);

    let prepared_name = fuse_pad_str(&String::from_utf8(name).unwrap(), true);

    let headerin = FuseInHeader {
        len: (prepared_name.len() as u32 + size_of::<FuseInHeader>() as u32),
        opcode: FuseOpcode::FuseLookup as u32,
        unique: 0,
        nodeid: nodeid,
        uid: 0,
        gid: 0,
        pid: 0,
        total_extlen: 0,
        padding: 0,
    };

    let headerin_bytes = headerin.as_bytes();
    let lookupin_bytes = prepared_name.as_slice();

    let headerout_buffer = [0u8; size_of::<FuseOutHeader>()];
    let lookupout_bytes = [0u8; size_of::<FuseEntryOut>()];
    let concat_req = [
        headerin_bytes,
        lookupin_bytes,
        &headerout_buffer,
        &lookupout_bytes,
    ]
    .concat();

    let mut reader = VmReader::from(concat_req.as_slice());
    let mut writer = self.request_buffers[0].writer().unwrap();
    let len = writer.write(&mut reader);
    let len_in = prepared_name.len() + size_of::<FuseInHeader>();

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

### FUSE_RELEASE

`FUSE_RELEASE` 接口用于关闭一个打开的文件。以下是该接口的实现步骤：

1. **输入**：
   - `FuseInHeader`：包含请求的元数据，包括请求长度、操作码、唯一标识符、节点 ID、用户 ID、组 ID、进程 ID 等。
   - `FuseReleaseIn`：包含关闭文件的参数，包括文件句柄、标志位和锁所有者。

2. **输出**：
   - `FuseOutHeader`：包含响应的元数据，包括响应长度、错误码、唯一标识符等。

```rust
fn release(&self, nodeid: u64, fh: u64, flags: u32, lock_owner: u64, flush: bool) {
    let mut request_queue = self.request_queues[0].disable_irq().lock();

    let headerin = FuseInHeader {
        len: (size_of::<FuseReleaseIn>() as u32 + size_of::<FuseInHeader>() as u32),
        opcode: FuseOpcode::FuseRelease as u32,
        unique: 0,
        nodeid: nodeid,
        uid: 0,
        gid: 0,
        pid: 0,
        total_extlen: 0,
        padding: 0,
    };

    let releasein = FuseReleaseIn {
        fh: fh,
        flags: flags,
        lock_owner: lock_owner,
        release_flags: if flush { FUSE_RELEASE_FLUSH } else { 0 },
    };

    let headerin_bytes = headerin.as_bytes();
    let releasein_bytes = releasein.as_bytes();
    let headerout_buffer = [0u8; size_of::<FuseOutHeader>()];
    let concat_req = [
        headerin_bytes,
        releasein_bytes,
        &headerout_buffer,
    ]
    .concat();

    let mut reader = VmReader::from(concat_req.as_slice());
    let mut writer = self.request_buffers[0].writer().unwrap();
    let len = writer.write(&mut reader);
    let len_in = size_of::<FuseReleaseIn>() + size_of::<FuseInHeader>();

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

### FUSE_ACCESS

`FUSE_ACCESS` 接口用于检查文件或目录的访问权限。以下是该接口的实现步骤：

1. **输入**：
   - `FuseInHeader`：包含请求的元数据，包括请求长度、操作码、唯一标识符、节点 ID、用户 ID、组 ID、进程 ID 等。
   - `FuseAccessIn`：包含访问检查的参数，包括节点 ID 和访问掩码。

2. **输出**：
   - `FuseOutHeader`：包含响应的元数据，包括响应长度、错误码、唯一标识符等。

```rust
fn access(&self, nodeid: u64, mask: u32) {
    let mut request_queue = self.request_queues[0].disable_irq().lock();

    let headerin = FuseInHeader {
        len: (size_of::<FuseAccessIn>() as u32 + size_of::<FuseInHeader>() as u32),
        opcode: FuseOpcode::FuseAccess as u32,
        unique: 0,
        nodeid: nodeid,
        uid: 0,
        gid: 0,
        pid: 0,
        total_extlen: 0,
        padding: 0,
    };

    let accessin = FuseAccessIn {
        mask: mask,
        padding: 0,
    };

    let headerin_bytes = headerin.as_bytes();
    let accessin_bytes = accessin.as_bytes();
    let headerout_buffer = [0u8; size_of::<FuseOutHeader>()];
    let accessout_bytes = [0u8; size_of::<FuseAttrOut>()];
    let concat_req = [
        headerin_bytes,
        accessin_bytes,
        &headerout_buffer,
        &accessout_bytes,
    ]
    .concat();

    let mut reader = VmReader::from(concat_req.as_slice());
    let mut writer = self.request_buffers[0].writer().unwrap();
    let len = writer.write(&mut reader);
    let len_in = size_of::<FuseAccessIn>() + size_of::<FuseInHeader>();

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

### FUSE_STATFS

`FUSE_STATFS` 接口用于获取文件系统的统计信息。以下是该接口的实现步骤：

1. **输入**：
   - `FuseInHeader`：包含请求的元数据，包括请求长度、操作码、唯一标识符、节点 ID、用户 ID、组 ID、进程 ID 等。

2. **输出**：
   - `FuseOutHeader`：包含响应的元数据，包括响应长度、错误码、唯一标识符等。
   - `FuseStatfsOut`：包含文件系统的统计信息，包括块数、空闲块数、可用块数、文件数、空闲文件数、块大小、最大文件名长度等。

```rust
fn statfs(&self, nodeid: u64) {
    let mut request_queue = self.request_queues[0].disable_irq().lock();

    let headerin = FuseInHeader {
        len: size_of::<FuseInHeader>() as u32,
        opcode: FuseOpcode::FuseStatfs as u32,
        unique: 0,
        nodeid: nodeid,
        uid: 0,
        gid: 0,
        pid: 0,
        total_extlen: 0,
        padding: 0,
    };

    let headerin_bytes = headerin.as_bytes();
    let headerout_buffer = [0u8; size_of::<FuseOutHeader>()];
    let statfsout_bytes = [0u8; size_of::<FuseStatfsOut>()];
    let concat_req = [headerin_bytes, &headerout_buffer, &statfsout_bytes].concat();

    let mut reader = VmReader::from(concat_req.as_slice());
    let mut writer = self.request_buffers[0].writer().unwrap();
    let len = writer.write(&mut reader);
    let len_in = size_of::<FuseInHeader>();

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

### FUSE_INTERRUPT

`FUSE_INTERRUPT` 接口用于中断一个正在进行的请求。以下是该接口的实现步骤：

1. **输入**：
   - `FuseInHeader`：包含请求的元数据，包括请求长度、操作码、唯一标识符、节点 ID、用户 ID、组 ID、进程 ID 等。
   - `FuseInterruptIn`：包含中断请求的参数，包括唯一标识符。

2. **输出**：
   - `FuseOutHeader`：包含响应的元数据，包括响应长度、错误码、唯一标识符等。

```rust
fn interrupt(&self, nodeid: u64, unique: u64) {
    let mut request_queue = self.request_queues[0].disable_irq().lock();

    let headerin = FuseInHeader {
        len: (size_of::<FuseInterruptIn>() as u32 + size_of::<FuseInHeader>() as u32),
        opcode: FuseOpcode::FuseInterrupt as u32,
        unique: 0,
        nodeid: nodeid,
        uid: 0,
        gid: 0,
        pid: 0,
        total_extlen: 0,
        padding: 0,
    };

    let interruptin = FuseInterruptIn { unique: unique };

    let headerin_bytes = headerin.as_bytes();
    let interruptin_bytes = interruptin.as_bytes();
    let headerout_buffer = [0u8; size_of::<FuseOutHeader>()];
    let concat_req = [headerin_bytes, interruptin_bytes, &headerout_buffer].concat();

    let mut reader = VmReader::from(concat_req.as_slice());
    let mut writer = self.request_buffers[0].writer().unwrap();
    let len = writer.write(&mut reader);
    let len_in = size_of::<FuseInterruptIn>() + size_of::<FuseInHeader>();

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

### FUSE_MKDIR

`FUSE_MKDIR` 接口用于创建一个新的目录。以下是该接口的实现步骤：

1. **输入**：
   - `FuseInHeader`：包含请求的元数据，包括请求长度、操作码、唯一标识符、节点 ID、用户 ID、组 ID、进程 ID 等。
   - `FuseMkdirIn`：包含创建目录的参数，包括模式和 umask。
   - 目录名称：包含要创建的目录的名称。

2. **输出**：
   - `FuseOutHeader`：包含响应的元数据，包括响应长度、错误码、唯一标识符等。
   - `FuseEntryOut`：包含新创建目录的元数据。

```rust
fn mkdir(&self, nodeid: u64, mode: u32, umask: u32, name: Vec<u8>) {
    let mut request_queue = self.request_queues[0].disable_irq().lock();

    let prepared_name = fuse_pad_str(&String::from_utf8(name).unwrap(), true);

    let headerin = FuseInHeader {
        len: (prepared_name.len() as u32 + size_of::<FuseMkdirIn>() as u32 + size_of::<FuseInHeader>() as u32),
        opcode: FuseOpcode::FuseMkdir as u32,
        unique: 0,
        nodeid: nodeid,
        uid: 0,
        gid: 0,
        pid: 0,
        total_extlen: 0,
        padding: 0,
    };

    let mkdirin = FuseMkdirIn {
        mode: mode,
        umask: umask,
    };

    let headerin_bytes = headerin.as_bytes();
    let mkdirin_bytes = mkdirin.as_bytes();
    let prepared_name_bytes = prepared_name.as_slice();

    let headerout_buffer = [0u8; size_of::<FuseOutHeader>()];
    let mkdirout_bytes = [0u8; size_of::<FuseEntryOut>()];
    let concat_req = [
        headerin_bytes,
        mkdirin_bytes,
        prepared_name_bytes,
        &headerout_buffer,
        &mkdirout_bytes,
    ]
    .concat();

    let mut reader = VmReader::from(concat_req.as_slice());
    let mut writer = self.request_buffers[0].writer().unwrap();
    let len = writer.write(&mut reader);
    let len_in = prepared_name.len() + size_of::<FuseMkdirIn>() + size_of::<FuseInHeader>();

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

### FUSE_CREATE

`FUSE_CREATE` 接口用于创建一个新的文件。以下是该接口的实现步骤：

1. **输入**：
   - `FuseInHeader`：包含请求的元数据，包括请求长度、操作码、唯一标识符、节点 ID、用户 ID、组 ID、进程 ID 等。
   - `FuseCreateIn`：包含创建文件的参数，包括模式和标志位。
   - 文件名：包含要创建的文件的名称。

2. **输出**：
   - `FuseOutHeader`：包含响应的元数据，包括响应长度、错误码、唯一标识符等。
   - `FuseEntryOut`：包含新创建文件的元数据。

```rust
fn create(&self, nodeid: u64, name: Vec<u8>, mode: u32, flags: u32) {
    let mut request_queue = self.request_queues[0].disable_irq().lock();

    let prepared_name = fuse_pad_str(&String::from_utf8(name).unwrap(), true);

    let headerin = FuseInHeader {
        len: (prepared_name.len() as u32 + size_of::<FuseCreateIn>() as u32 + size_of::<FuseInHeader>() as u32),
        opcode: FuseOpcode::FuseCreate as u32,
        unique: 0,
        nodeid: nodeid,
        uid: 0,
        gid: 0,
        pid: 0,
        total_extlen: 0,
        padding: 0,
    };

    let createin = FuseCreateIn {
        flags: flags,
        mode: mode,
        umask: 0,
    };

    let headerin_bytes = headerin.as_bytes();
    let createin_bytes = createin.as_bytes();
    let prepared_name_bytes = prepared_name.as_slice();

    let headerout_buffer = [0u8; size_of::<FuseOutHeader>()];
    let createout_bytes = [0u8; size_of::<FuseEntryOut>()];
    let concat_req = [
        headerin_bytes,
        createin_bytes,
        prepared_name_bytes,
        &headerout_buffer,
        &createout_bytes,
    ]
    .concat();

    let mut reader = VmReader::from(concat_req.as_slice());
    let mut writer = self.request_buffers[0].writer().unwrap();
    let len = writer.write(&mut reader);
    let len_in = prepared_name.len() + size_of::<FuseCreateIn>() + size_of::<FuseInHeader>();

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

### FUSE_FORGET

`FUSE_FORGET` 接口用于通知文件系统内核模块忘记一个 inode。以下是该接口的实现步骤：

1. **输入**：
   - `FuseInHeader`：包含请求的元数据，包括请求长度、操作码、唯一标识符、节点 ID、用户 ID、组 ID、进程 ID 等。
   - `FuseForgetIn`：包含忘记 inode 的参数，包括引用计数。

2. **输出**：
   - `FuseOutHeader`：包含响应的元数据，包括响应长度、错误码、唯一标识符等。

```rust
fn forget(&self, nodeid: u64, nlookup: u64) {
    let mut hiprio_queue = self.hiprio_queue.disable_irq().lock();

    let headerin = FuseInHeader {
        len: (size_of::<FuseForgetIn>() as u32 + size_of::<FuseInHeader>() as u32),
        opcode: FuseOpcode::FuseForget as u32,
        unique: 0,
        nodeid: nodeid,
        uid: 0,
        gid: 0,
        pid: 0,
        total_extlen: 0,
        padding: 0,
    };

    let forgetin = FuseForgetIn { nlookup: nlookup };

    let headerin_bytes = headerin.as_bytes();
    let forgetin_bytes = forgetin.as_bytes();
    let headerout_buffer = [0u8; size_of::<FuseOutHeader>()];
    let concat_req = [headerin_bytes, forgetin_bytes, &headerout_buffer].concat();

    let mut reader = VmReader::from(concat_req.as_slice());
    let mut writer = self.request_buffers[0].writer().unwrap();
    let len = writer.write(&mut reader);
    let len_in = size_of::<FuseForgetIn>() + size_of::<FuseInHeader>();

    self.request_buffers[0].sync(0..len).unwrap();
    let slice_in = DmaStreamSlice::new(&self.request_buffers[0], 0, len_in);
    let slice_out = DmaStreamSlice::new(&self.request_buffers[0], len_in, len);

    hiprio_queue
        .add_dma_buf(&[&slice_in], &[&slice_out])
        .unwrap();

    if hiprio_queue.should_notify() {
        hiprio_queue.notify();
    }
}
```

### FUSE_BATCH_FORGET

`FUSE_BATCH_FORGET` 接口用于批量通知文件系统内核模块忘记多个 inode。以下是该接口的实现步骤：

1. **输入**：
   - `FuseInHeader`：包含请求的元数据，包括请求长度、操作码、唯一标识符、节点 ID、用户 ID、组 ID、进程 ID 等。
   - `FuseBatchForgetIn`：包含批量忘记 inode 的参数，包括 inode 数量。
   - `FuseForgetOne`：包含每个 inode 的节点 ID 和引用计数。

2. **输出**：
   - `FuseOutHeader`：包含响应的元数据，包括响应长度、错误码、唯一标识符等。

```rust
fn batch_forget(&self, forget_list: &[(u64, u64)]) {
    let mut hiprio_queue = self.hiprio_queue.disable_irq().lock();

    let headerin = FuseInHeader {
        len: (forget_list.len() * size_of::<FuseForgetOne>() + size_of::<FuseInHeader>()) as u32,
        opcode: FuseOpcode::FuseBatchForget as u32,
        unique: 0,
        nodeid: 0,
        uid: 0,
        gid: 0,
        pid: 0,
        total_extlen: 0,
        padding: 0,
    };

    let mut forgetin_bytes = Vec::new();
    for (nodeid, nlookup) in forget_list {
        let forget_one = FuseForgetOne {
            nodeid: *nodeid,
            nlookup: *nlookup,
        };
        forgetin_bytes.extend_from_slice(&forget_one.as_bytes());
    }

    let headerin_bytes = headerin.as_bytes();
    let headerout_buffer = [0u8; size_of::<FuseOutHeader>()];
    let concat_req = [headerin_bytes, &forgetin_bytes, &headerout_buffer].concat();

    let mut reader = VmReader::from(concat_req.as_slice());
    let mut writer = self.request_buffers[0].writer().unwrap();
    let len = writer.write(&mut reader);
    let len_in = forget_list.len() * size_of::<FuseForgetOne>() + size_of::<FuseInHeader>();

    self.request_buffers[0].sync(0..len).unwrap();
    let slice_in = DmaStreamSlice::new(&self.request_buffers[0], 0, len_in);
    let slice_out = DmaStreamSlice::new(&self.request_buffers[0], len_in, len);

    hiprio_queue
        .add_dma_buf(&[&slice_in], &[&slice_out])
        .unwrap();

    if hiprio_queue.should_notify() {
        hiprio_queue.notify();
    }
}
```

### FUSE_WRITE

`FUSE_WRITE` 接口用于写入文件内容。以下是该接口的实现步骤：

1. **输入**：
   - `FuseInHeader`：包含请求的元数据，包括请求长度、操作码、唯一标识符、节点 ID、用户 ID、组 ID、进程 ID 等。
   - `FuseWriteIn`：包含写入文件的参数，包括文件句柄、偏移量、写入大小、写入标志、锁所有者和标志。
   - 数据：包含要写入文件的实际数据。

2. **输出**：
   - `FuseOutHeader`：包含响应的元数据，包括响应长度、错误码、唯一标识符等。
   - `FuseWriteOut`：包含写入操作的结果，包括写入的字节数。

```rust
fn write(&self, nodeid: u64, fh: u64, offset: u64, data: &[u8]) {
    let mut request_queue = self.request_queues[0].disable_irq().lock();

    let data = [data, vec![0u8; (8 - (data.len() & 0x7)) & 0x7].as_slice()].concat();

    let headerin = FuseInHeader {
        len: (size_of::<FuseWriteIn>() as u32 + size_of::<FuseInHeader>() as u32 + data.len() as u32),
        opcode: FuseOpcode::FuseWrite as u32,
        unique: 0,
        nodeid: nodeid,
        uid: 0,
        gid: 0,
        pid: 0,
        total_extlen: 0,
        padding: 0,
    };

    let writein = FuseWriteIn {
        fh: fh,
        offset: offset,
        size: data.len() as u32,
        write_flags: 0,
        lock_owner: 0,
        flags: 0,
        padding: 0,
    };

    let headerin_bytes = headerin.as_bytes();
    let writein_bytes = writein.as_bytes();
    let data_bytes = data.as_slice();
    let headerout_buffer = [0u8; size_of::<FuseOutHeader>()];
    let writeout_buffer = [0u8; size_of::<FuseWriteOut>()];
    let concat_req = [
        headerin_bytes,
        writein_bytes,
        data_bytes,
        &headerout_buffer,
        &writeout_buffer,
    ]
    .concat();

    let mut reader = VmReader::from(concat_req.as_slice());
    let mut writer = self.request_buffers[0].writer().unwrap();
    let len = writer.write(&mut reader);
    let len_in = size_of::<FuseWriteIn>() + size_of::<FuseInHeader>() + data.len() as usize;

    self.request_buffers[0].sync(0..len).unwrap();
    let slice_in = DmaStreamSlice::new(&self.request_buffers[0], 0, len_in as usize);
    let slice_out = DmaStreamSlice::new(&self.request_buffers[0], len_in as usize, len);

    request_queue
        .add_dma_buf(&[&slice_in], &[&slice_out])
        .unwrap();

    if request_queue.should_notify() {
        request_queue.notify();
    }
}
```

### FUSE_DESTROY

`FUSE_DESTROY` 接口用于销毁文件系统实例。以下是该接口的实现步骤：

1. **输入**：
   - `FuseInHeader`：包含请求的元数据，包括请求长度、操作码、唯一标识符、节点 ID、用户 ID、组 ID、进程 ID 等。

2. **输出**：
   - `FuseOutHeader`：包含响应的元数据，包括响应长度、错误码、唯一标识符等。

```rust
fn destroy(&self, nodeid: u64) {
    let mut request_queue = self.request_queues[0].disable_irq().lock();

    let headerin = FuseInHeader {
        len: size_of::<FuseInHeader>() as u32,
        opcode: FuseOpcode::FuseDestroy as u32,
        unique: 0,
        nodeid: nodeid,
        uid: 0,
        gid: 0,
        pid: 0,
        total_extlen: 0,
        padding: 0,
    };

    let headerin_bytes = headerin.as_bytes();
    let headerout_buffer = [0u8; size_of::<FuseOutHeader>()];
    let concat_req = [headerin_bytes, &headerout_buffer].concat();

    let mut reader = VmReader::from(concat_req.as_slice());
    let mut writer = self.request_buffers[0].writer().unwrap();
    let len = writer.write(&mut reader);
    let len_in = size_of::<FuseInHeader>();

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

### FUSE_RENAME

`FUSE_RENAME` 接口用于重命名文件或目录。以下是该接口的实现步骤：

1. **输入**：
   - `FuseInHeader`：包含请求的元数据，包括请求长度、操作码、唯一标识符、节点 ID、用户 ID、组 ID、进程 ID 等。
   - `FuseRenameIn`：包含重命名的参数，包括新目录的节点 ID。
   - 原名称：包含要重命名的文件或目录的原名称。
   - 新名称：包含重命名后的文件或目录的新名称。

2. **输出**：
   - `FuseOutHeader`：包含响应的元数据，包括响应长度、错误码、唯一标识符等。

```rust
fn rename(&self, nodeid: u64, name: Vec<u8>, newdir: u64, newname: Vec<u8>) {
    let mut request_queue = self.request_queues[0].disable_irq().lock();

    let prepared_name = fuse_pad_str(&String::from_utf8(name).unwrap(), true);
    let prepared_newname = fuse_pad_str(&String::from_utf8(newname).unwrap(), true);

    let headerin = FuseInHeader {
        len: (size_of::<FuseRenameIn>() as u32 + size_of::<FuseInHeader>() as u32 + prepared_name.len() as u32 + prepared_newname.len() as u32),
        opcode: FuseOpcode::FuseRename as u32,
        unique: 0,
        nodeid: nodeid,
        uid: 0,
        gid: 0,
        pid: 0,
        total_extlen: 0,
        padding: 0,
    };

    let renamein = FuseRenameIn { newdir: newdir };

    let headerin_bytes = headerin.as_bytes();
    let renamein_bytes = renamein.as_bytes();
    let prepared_name_bytes = prepared_name.as_slice();
    let prepared_newname_bytes = prepared_newname.as_slice();

    let headerout_buffer = [0u8; size_of::<FuseOutHeader>()];
    let concat_req = [
        headerin_bytes,
        renamein_bytes,
        prepared_name_bytes,
        prepared_newname_bytes,
        &headerout_buffer,
    ]
    .concat();

    let mut reader = VmReader::from(concat_req.as_slice());
    let mut writer = self.request_buffers[0].writer().unwrap();
    let len = writer.write(&mut reader);
    let len_in = size_of::<FuseRenameIn>() + size_of::<FuseInHeader>() + prepared_name.len() + prepared_newname.len();

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

### FUSE_RENAME2

`FUSE_RENAME2` 接口用于重命名文件或目录，并允许指定额外的标志。以下是该接口的实现步骤：

1. **输入**：
   - `FuseInHeader`：包含请求的元数据，包括请求长度、操作码、唯一标识符、节点 ID、用户 ID、组 ID、进程 ID 等。
   - `FuseRename2In`：包含重命名的参数，包括新目录的节点 ID和标志位。
   - 原名称：包含要重命名的文件或目录的原名称。
   - 新名称：包含重命名后的文件或目录的新名称。

2. **输出**：
   - `FuseOutHeader`：包含响应的元数据，包括响应长度、错误码、唯一标识符等。

```rust
fn rename2(&self, nodeid: u64, name: Vec<u8>, newdir: u64, newname: Vec<u8>, flags: u32) {
    let mut request_queue = self.request_queues[0].disable_irq().lock();

    let prepared_name = fuse_pad_str(&String::from_utf8(name).unwrap(), true);
    let prepared_newname = fuse_pad_str(&String::from_utf8(newname).unwrap(), true);

    let headerin = FuseInHeader {
        len: (size_of::<FuseRename2In>() as u32 + size_of::<FuseInHeader>() as u32 + prepared_name.len() as u32 + prepared_newname.len() as u32),
        opcode: FuseOpcode::FuseRename2 as u32,
        unique: 0,
        nodeid: nodeid,
        uid: 0,
        gid: 0,
        pid: 0,
        total_extlen: 0,
        padding: 0,
    };

    let rename2in = FuseRename2In {
        newdir: newdir,
        flags: flags,
    };

    let headerin_bytes = headerin.as_bytes();
    let rename2in_bytes = rename2in.as_bytes();
    let prepared_name_bytes = prepared_name.as_slice();
    let prepared_newname_bytes = prepared_newname.as_slice();

    let headerout_buffer = [0u8; size_of::<FuseOutHeader>()];
    let rename2out_bytes = [0u8; size_of::<FuseEntryOut>()];
    let concat_req = [
        headerin_bytes,
        rename2in_bytes,
        prepared_name_bytes,
        prepared_newname_bytes,
        &headerout_buffer,
        &rename2out_bytes,
    ]
    .concat();

    let mut reader = VmReader::from(concat_req.as_slice());
    let mut writer = self.request_buffers[0].writer().unwrap();
    let len = writer.write(&mut reader);
    let len_in = size_of::<FuseRename2In>() + size_of::<FuseInHeader>() + prepared_name.len() + prepared_newname.len();

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

