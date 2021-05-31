# proj4-SUSTech-xv6tql

所选题目ID: proj4

所选题目内容：采用Rust重新实现xv6

团队成员：
- 江川(框架搭建、内核开发)
- 聂湘林(用户程序开发)
- 邹泽桦(内核开发、用户程序开发)

## 项目介绍

xv6 是 MIT 开发的教学操作系统，广泛地应用在如 6.S081 等操作系统课程中。Rust 是用于构建可靠、高效软件的现代编程语言，Rust 提供的许多软件特性如所有权等机制可以有效保障软件的可靠性。

本项目旨在用 Rust 实现 xv6。我们希望尽可能地使用 Rust 的语言特征，在确保操作系统的可靠性的同时减少人工检查。我们将使用 Rust 完成 Project，用 cargo 作为构建工具，使用 QEMU 模拟硬件设备。

## 构建方法

推荐系统：Ubuntu 20.04

### 安装 [QEMU](https://www.qemu.org/)

在终端中执行：

```shell
sudo apt-get install git gdb-multiarch qemu-system-misc binutils-riscv64-unknown-elf build-essential pkg-config libssl-dev
```

如果使用 Ubuntu 18，不需要安装 qemu-system-misc，需要另行安装 [QEMU4.2](https://www.qemu.org/download/#source) 以上。

### 安装 [Rust](https://www.rust-lang.org/)

终端运行 `curl https://sh.rustup.rs -sSf | sh` 安装 rustup，需要修改 toolchain 为 nightly。

在终端中执行：

```shell
source $HOME/.cargo/env
rustup target add riscv64gc-unknown-none-elf
cargo install --force cargo-make
```

### 运行本项目

直接运行：`cargo make run`

启用 gdb 调试：`cargo make debug`

## 功能描述

本项目我们最终实现了如下功能

- Driver
    - UART
    - VirtIO
- Memory
    - Virtual Memory
    - Kernel Heap
- Lock
    - Spin Lock
    - Sleep Lock
- Process
    - Process
    - Schedule
    - System Call
- File System
    - Block Cache Layer
    - Log Layer
    - iNode File System
- User Space Program and Library
    - init
    - Shell
    - other user programs like ls, cat
    - mkfs: linux program that build iso files for file system


## 设计思路

### 构建工具

由于本项目构建了多个运行在不同怕平台上的程序，如运行在 riscv-unknown-none-elf 平台的 kernel、shell、ls 等，以及 x86-64-unknown-linux 平台的用于构建文件系统的 mkfs，为了共享所用到的一些公共文件，本项目将代码划分才多个 cargo crate 中，使用 cargo workspace 进行管理。

为了自定义对不同平台的程序采用不同的编译参数，我们使用了 cargo-make 增强 cargo 的构建系统，引入了 `Makefile.toml` 以类似 `make` 的方式管理编译参数。

### 内核模块划分

根据内核的不同功能，我们将内核划分为不同的模块

- memory: 内存模块，有 PhysicalMemory、PageTable、KernelHeap 等子模块
- driver: 驱动模块，包含 UART 和 VertIO 的驱动
- file_system: 文件系统模块，包括 BlockCache、Log、iNode、Path 等多层实现
- process: 进程模块，包括 Process、schedule、context switch 等抽象与实现。
- other: print、spin_lock、riscv 等辅助模块

## 实现描述

### OS启动流程

Qemu 启动后，jump 到 entry.S 中的 \_entry 代码段，\_entry 会给每个 CPU 分配 8192 字节的栈空间，随后 jump 到 Rust 写的 start 函数中。start 函数设置了 Supervisor mode 并初始化了 timer 等中断后，进入 main 函数。

在 main 中我们依次对如下模块进行初始化操作

1. 初始化 UART 模块，使得 print 可用
2. 识别可用的物理内存大小，进行分页等初始化操作
3. 创建并映射了 Kernel Page Table，切换到虚拟地址空间
4. 初始化 Kernel Heap，使得 Vec、Box 等依赖于堆的数据结构可用
5. 初始化 Process
6. 注册处理 Trap 的 handler
7. 初始化 PLIC
8. 读取 SuperBlock，初始化 FileSystem
9. 创建第一个进程 init
10. 开启 Interrupt，调用 scheduler 执行进程

### Memory模块

Memory模块提供了对内存功能的抽象。其中 `physical_memory` 负责管理以 Page 为单位的物理内存。`virtual_memory` 提供了基于 Sv39 系统的三级 PageTable。`kernel_virtual_memory` 和 `user_virtual_memory` 封装了常用的管理内核和用户虚拟地址空间的方法。

这个模块大多数设计与原本的 xv6 差不多，但我们仍进行了以下变更。

#### 重构 PageTable

xv6 的 walk 函数返回了虚拟地址对应的 PTE，但这实际上是一个破环封装性的方法。经过研究我们重构了 PageTable，使其对外提供 `map`、`unmap`、`translate`、`get_flag`、`set_flag` 等必要功能，其他扩展功能均可以通过如上的抽象实现。

另外，为了充分运用 Rust 类型系统的检查，我们将三级树结构中的每一级设为了不同的类型，运用泛型为每一级生成重复的代码。从类型系统上根本地解决了修改不同级的树出错的情况。

#### 添加 Kernel Heap

在内存映射上，我们的 kernel 比 xv6 多了一块，在虚拟地址 0x40000000 的位置分配了 1MB 的空间，用于实现 Kernel Heap。我们使用了 [linked_list_allocator](https://crates.io/crates/linked_list_allocator) 作为 Kernel Heap 的 allocator。将其标记为 Rust alloc 库中的 `global_allocator` 后，便可以使用如 `Vec`、`Box`、`String` 等依赖堆的常用数据结构，简化后面的开发。

#### 扩大 Kernel Stack

我们将 Kernel Stack 从 xv6 的 1 Page (4096 bytes) 扩大为了 4 Pages (16384 bytes)，详见 [遇见的困难 · Kernel Stack 过小](#遇见的困难) 一节。

### Driver模块

Driver模块主要实现了 `uart` 和 `virtio_disk` 两个驱动。

`uart` 提供了读写数据的功能，qemu 在模拟时会用一个 terminal 进行交互。uart 的写数据分为异步和同步两种，异步的用于用户态，同步的用于内核态，并有一个32字节的缓冲区。读则是通过 interrupt 并将输入的字符发送给上层的 console 模块，console 根据输入的字符进行响应。

`virtio_disk` 则是一个基于 VirtIO 的读写 Block 的模块，用于读写硬盘，qemu 会加载 `fs.img` 提供给 VirtIO 使用。在 xv6 中默认的 Block Size 是 1024 bytes。

### File System 模块

在项目中，我们完整地实现了 xv6 中提到的多个于文件系统相关的部分，包括 `path`、`inode`、`logging`、`buffer_cache` 等多个 Layer。

位于最底层的是 `buffer_cache` ，这层直接于 `virtio_disk` 驱动进行交互，并提供了一个 Cache，用于加速 IO。在这一层中我们使用了 LRU 算法，用于置换过旧的 Cache。

随后是 `logging` 层，这一层的目的在于保护文件一致性。提供了类似事务的功能，可以在断电等极端情况也能保证，要么 IO 操作完整发生，要么完全不发生。在内核初始化 File System 时这层会检查文件系统中是否存在未完成的事务，并进行恢复。

然后文件的抽象 `inode` 。`inode` 使用了一个 Block 存放文件的 MetaData，以及10个直接引用和一个一级间接引用存放文件的数据。`inode` 这层我们遇到了单文件大小限制过小的问题，详见 [遇见的困难 · File System 支持的单文件大小过小](#遇到的问题) 一节。 

之后是顶层用于表示文件夹和目录的 `Path`、`Directory` 抽象程度较高的层，用于为文件系统提供更高度抽象的服务。

### Process模块

Process 模块主要提供 Process 以及 CPU 的抽象。

`process` 提供了 `sleep`、`wakeup` 等转换自身状态的方法，`process_manager` 则提供了分配释放进程、调度、`fork`、`exit`等管理进程的方法，以及一个简易的 `schedule` 用于调度进程。

### 其他内核模块

`spin_lock` 是一个自旋锁模块，通过关闭中断和原子的读写操作来保持正确性。由于自旋锁通常只会短期持有、等待，因此我们运用 Rust 的特性实现了一个 `SpinLockGuard`，使得这个变量回收时会自动释放锁。

`sleep_lock` 是一个基于 `sleep`、`wakeup`、`spin_lock` 实现的锁模块。

`print` 为内核编写提供了几个常用的宏 `println`、`print`、`assert`以及默认的 panic handler，通过 `uart` 同步地输出调试信息。

`riscv` 则是对 Risc-V 进行的抽象，提供了一些读写寄存器、刷新 TLB 等底层的功能。

`syscall` 封装了内核的功能，作为 System Call 提供给用户态的程序。

`trap` 则是统一的 Interrupt、Exception 的 handler，会根据具体的 trap 种类分发到 `driver` 或 `syscall` 进行处理。

### 用户态的程序和库

`mkfs` 是一个运行在 linux 上的用户程序，它的功能是将编译生成的用户程序以及其他文件，打包成一个 xv6 可识别的文件系统的 iso 镜像。我们在 xv6 的基础上做了改进，能够将文件放置到指定的目录中。

为了简化用户态程序的开发，我们为用户态的程序创建了一个 library。它提供的功能有：

- 提供 System Call 的函数封装
- 提供 print!、fprint! 等常用 rust 宏
- 提供 malloc、free 以及对应的 rust 封装的 allocator，使得用户态程序可以直接使用依赖于堆的数据结构
- 提供程序入口的 entry，会将程序运行的 c 风格的参数列表转化为 rust 风格的 `Vec<&str>` ，并调用用户程序的 `main`，在 `main` 结束后执行默认的 `exit` 操作

随后，我们实现了几个用户态程序

`init` 是 kernel 启动的第一个进程，它会创建 console 的设备文件，并启动 `sh`。它会始终运行，不断 `wait` 为 `reparent` 后终止的进程回收资源。

`sh` 提供基本功能的 `shell`。

`ls` 简单的用户态程序，会列出文件夹内所有文件的文件名、大小、文件类型等信息。

`cat` 查看文件内容。

## 遇到的问题

### Kernel Stack 过小

在开发过程中，我们发现有时执行 `Vec.push()` 方法时会产生 Kernel Trap，查询 `scause` 寄存器会发现这是一个 `Store/AMO page fault`。遇到这个问题我们团队第一反应是 Kernel Heap 出了问题，由于 Kernel Heap 是 xv6 中没有的，是我们新增的，因此我们在开始的很长一段时间都在怀疑是否是 Kernel Heap 实现错了错误。但如果我们尝试提前用 `Vec.reserve()` 预留足够的空间，这个 Trap 又消失了。

直到我们后来注意到了 `stval` 寄存器显示了一个很大的地址如 0xfffffe00，而这个位置是 kernel virtual memory 中映射 kernel stack 所在的位置，我们开始怀疑这是一个栈溢出的问题。果然，我们将 kernel stack 从 1 page 增大到 4 pages 后，这个问题就再也没有出现过了。推测这是由于 `push` 中检测到空间不够、又尝试扩容的过程中，出现了过多函数调用和局部变量导致的。

### File System 支持的单文件大小过小

xv6 的 inode 中有 12 个直接地址项和 1 个间接地址项，Block Size 为 1024.所以一个 inode 最多能指向 `12 + 1024/4 = 268` 个 Block，即单文件最大大小为 `268 * 1024B = 268KB`。

相比起 xv6 中使用的 C 语言，Rust 编译出的可执行文件涵盖了许多符号信息和调试信息，所以文件大小容易超出 268KB。因此，我们在将可执行文件编译完成后，用 `riscv64-unknown-elf-strip` 删除掉一些符号信息和调试信息，减小文件的体积。

但这并不是最好的解决方案，`strip` 会使得用户态的程序难以调试。我们计划未来为 xv6_rust 增加1个二级指针，这会使得 xv6 支持大约 64MB 大小的文件，或是扩大默认为 1024字节的 Block Size。但无论选择哪种解决方案，都会使得我们的 xv6_rust 于原版的 xv6 在文件系统上出现不兼容的状况。

## Future Works

在开发过程中，由于对一些领域接触的比较少，部分代码较多地参考了 C 代码，使得我们的 Rust 风格非常类似 C，充斥着大量 unsafe 和不够 Rustful 的代码。虽然有少数代码经过精心的重构后（如Memory模块）更加符合 Rust 的规范，充分利用了 Rust 的语言特性，但仍然有很多部分是不足的，因此重构是很有必要的。

另外，现在的 xv6_rust 虽然已经可以使用，但未经过充分测试，仍可能存在不少 bug。为了解决这个问题，我们可能需要像 xv6 那样实现一个 usertest 的用户程序，用来检测潜在的bug。

当确定了系统的可靠性后，我们或许可以开始移植 MIT 6.S081 课程的实验内容，并添加新的模块（如网络），使得本项目能有更多教学价值。

## 总结

整个操作系统的开发是一项巨大的工程，在进行这个项目的过程中，我们小组充分的意识到了这一点。这次项目使得我们对操作系统有了更充分的认识，没有什么比亲手写一个操作系统更好的熟悉操作系统的方式了。虽然我们都是第一次写操作系统、对 Rust 也不太熟悉，踩了不少坑，但仍然收获良多。
