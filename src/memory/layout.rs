pub const KERNEL_BASE: usize = 0x80000000;
pub const PHY_STOP: usize = KERNEL_BASE + 128 * 1024 * 1024;
pub const MAX_VA: usize = 1 << ((9 + 9 + 9 + 12) - 1);