use spin::Mutex;

use crate::process::cpu_id;
use crate::riscv::{intr_get, read_scause, read_sepc, read_sip, read_sstatus, read_stval, SSTATUS_SPP, write_sepc, write_sip, write_sstatus, write_stvec};

pub static TICKS: Mutex<usize> = Mutex::new(0);

extern {
    fn kernelvec();
}

pub fn hard_init() {
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

    // TODO yield

    write_sepc(sepc);
    write_sstatus(sstatus);
}

unsafe fn clock_intr() {
    *TICKS.lock() += 1;
}

// check if it's an external interrupt or software interrupt,
// and handle it.
// returns 2 if timer interrupt,
// 1 if other device,
// 0 if not recognized.
unsafe fn dev_intr() -> usize {
    let scause = read_scause();
    if scause & 0x8000000000000000 != 0 && scause & 0xff == 9 {
        // this is a supervisor external interrupt, via PLIC.

        // irq indicates which device interrupted.
        // TODO
        // let irq = plic_claim();

        return 1;
    } else if scause == 0x8000000000000001 {
        // software interrupt from a machine-mode timer interrupt,
        // forwarded by timervec in kernelvec.S.

        if cpu_id() == 0 {
            clock_intr();
        }

        // acknowledge the software interrupt by clearing
        // the SSIP bit in sip.
        write_sip(read_sip() & !2);

        return 2;
    } else {
        return 0;
    }
}