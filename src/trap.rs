use spin::Mutex;

use crate::console::uart::uart_intr;
use crate::driver::DISK;
use crate::memory::layout::{UART0_IRQ, VIRTIO0_IRQ};
use crate::plic::{plic_claim, plic_complete};
use crate::process::{cpu_id, CPU_MANAGER, PROCESS_MANAGER};
use crate::process::process::ProcessState::RUNNING;
use crate::riscv::{intr_get, read_scause, read_sepc, read_sip, read_sstatus, read_stval, SSTATUS_SPP, write_sepc, write_sip, write_sstatus, write_stvec};

pub static TICKS: Mutex<usize> = Mutex::new(0);

extern {
    fn kernelvec();
}

pub fn trap_hart_init() {
    unsafe {
        write_stvec(kernelvec as usize);
    }
}

#[no_mangle]
pub unsafe fn kerneltrap() {
    let sepc = read_sepc();
    let sstatus = read_sstatus();
    let scause = read_scause();

    if (sstatus & SSTATUS_SPP) == 0 {
        panic!("kerneltrap: not from supervisor mode");
    }

    if intr_get() {
        panic!("kerneltrap: interrupts enabled");
    }

    let which_dev = dev_intr();
    if which_dev == 0 {
        println!("scause {}", scause);
        println!("sepc={} stval={}", read_sepc(), read_stval());
        panic!("kerneltrap");
    }

    let cpu = CPU_MANAGER.my_cpu_mut();
    let process = cpu.my_proc();
    if which_dev == 2 && !process.is_null() && process.as_ref().unwrap().info.lock().state == RUNNING {
        cpu.yield_self();
    }

    write_sepc(sepc);
    write_sstatus(sstatus);
}

unsafe fn clock_intr() {
    *TICKS.lock() += 1;
    PROCESS_MANAGER.wakeup(&TICKS as *const _ as usize)
}

// check if it's an external interrupt or software interrupt,
// and handle it.
// returns 2 if timer interrupt,
// 1 if other device,
// 0 if not recognized.
unsafe fn dev_intr() -> usize {
    let scause = read_scause();
    return if scause & 0x8000000000000000 != 0 && scause & 0xff == 9 {
        // this is a supervisor external interrupt, via PLIC.

        // irq indicates which device interrupted.
        let irq = plic_claim();

        if irq as usize == UART0_IRQ {
            uart_intr();
        } else if irq as usize == VIRTIO0_IRQ {
            DISK.intr();
        } else if irq != 0 {
            println!("unexpected interrupt irq={}", irq);
        }

        // the PLIC allows each device to raise at most one
        // interrupt at a time; tell the PLIC the device is
        // now allowed to interrupt again.
        if irq != 0 {
            plic_complete(irq);
        }

        1
    } else if scause == 0x8000000000000001 {
        // software interrupt from a machine-mode timer interrupt,
        // forwarded by timervec in kernelvec.S.

        if cpu_id() == 0 {
            clock_intr();
        }

        // acknowledge the software interrupt by clearing
        // the SSIP bit in sip.
        write_sip(read_sip() & !2);

        2
    } else {
        0
    };
}