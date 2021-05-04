use crate::memory::layout::{PLIC, UART0_IRQ, VIRTIO0_IRQ};
use crate::process::cpu_id;

pub fn plic_init() {
    unsafe {
        *((PLIC + UART0_IRQ * 4) as *mut u32) = 1;
        *((PLIC + VIRTIO0_IRQ * 4) as *mut u32) = 1;
    }
}

pub fn plic_hart_init() {
    let hart = cpu_id();

    let plic_senable = PLIC + 0x2080 + hart * 0x100;
    let plic_spriority = PLIC + 0x201000 + hart * 0x2000;

    unsafe {
        *(plic_senable as *mut u32) = 1 << UART0_IRQ | 1 << VIRTIO0_IRQ;
        *(plic_spriority as *mut u32) = 0;
    }
}

pub fn plic_claim() -> u32 {
    unsafe {
        let hart = cpu_id();
        let irq = *((PLIC + 0x201004 + hart * 0x2000) as *mut u32);
        return irq;
    }
}

pub fn plic_complete(irq: u32) {
    unsafe {
        let hart = cpu_id();
        *((PLIC + 0x201004 + hart * 0x2000) as *mut u32) = irq;
    }
}