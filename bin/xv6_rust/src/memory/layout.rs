use crate::memory::PAGE_SIZE;

pub const KERNEL_BASE: usize = 0x80000000;
pub const PHY_STOP: usize = KERNEL_BASE + 128 * 1024 * 1024;
pub const MAX_VA: usize = 1 << ((9 + 9 + 9 + 12) - 1);

pub const UART0: usize = 0x10000000;
pub const UART0_IRQ: usize = 10;

pub const VIRTIO0: usize = 0x10001000;
pub const VIRTIO0_IRQ: usize = 1;

pub const CLINT: usize = 0x2000000;
pub const PLIC: usize = 0x0c000000;
pub const TRAMPOLINE: usize = MAX_VA - PAGE_SIZE;
pub const TRAPFRAME: usize = TRAMPOLINE - PAGE_SIZE;

pub const KERNEL_HEAP_START: usize = 0x40000000;
pub const KERNEL_HEAP_SIZE: usize = 1 * 1024 * 1024;

pub const KERNEL_STACK_PAGE_COUNT: usize = 4;
