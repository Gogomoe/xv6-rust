use core::mem::transmute;

use crate::console::uart::uart_intr;
use crate::driver::DISK;
use crate::memory::{make_satp, PAGE_SIZE};
use crate::memory::layout::{TRAMPOLINE, TRAPFRAME, UART0_IRQ, VIRTIO0_IRQ};
use crate::plic::{plic_claim, plic_complete};
use crate::process::{cpu_id, CPU_MANAGER, PROCESS_MANAGER};
use crate::process::process::ProcessState::RUNNING;
use crate::riscv::{intr_get, intr_off, read_satp, read_scause, read_sepc, read_sip, read_sstatus, read_stval, read_tp, SSTATUS_SPIE, SSTATUS_SPP, write_sepc, write_sip, write_sstatus, write_stvec};
use crate::spin_lock::SpinLock;

pub static TICKS: SpinLock<usize> = SpinLock::new(0, "ticks");

extern {
    fn kernelvec();
    fn uservec();
    fn trampoline();
    fn userret();
}

pub fn trap_hart_init() {
    unsafe {
        write_stvec(kernelvec as usize);
    }
}

#[no_mangle]
pub unsafe fn usertrap() {
    if read_sstatus() & SSTATUS_SPP != 0 {
        panic!("not from user mode");
    }

    // send interrupts and exceptions to kerneltrap(),
    // since we're now in the kernel.
    let kernelvec = kernelvec as usize;
    write_stvec(kernelvec);

    let process = CPU_MANAGER.my_proc().as_ref().unwrap();
    let data = process.data.get().as_mut().unwrap();
    let trap_frame = data.trap_frame.as_mut().unwrap();

    // save user program counter.
    trap_frame.epc = read_sepc() as u64;

    let which_dev = dev_intr();

    if read_scause() == 8 {
        // system call
    } else if which_dev != 0 {
        // ok
    } else {
        println!("unexpected scause {:x} pid={}", read_scause(), (*process.info.lock()).pid);
        println!("sepc={:x} stval={:x}", read_sepc(), read_stval());
        // TODO kill p
    }

    todo!()
}

pub unsafe fn user_trap_return() {
    let process = CPU_MANAGER.my_proc().as_ref().unwrap();

    // we're about to switch the destination of traps from
    // kerneltrap() to usertrap(), so turn off interrupts until
    // we're back in user space, where usertrap() is correct.
    intr_off();

    // send syscalls, interrupts, and exceptions to trampoline.S
    let uservec = uservec as usize;
    let trampoline = trampoline as usize;
    let userret = userret as usize;
    write_stvec(TRAMPOLINE + (uservec - trampoline));

    // set up trapframe values that uservec will need when
    // the process next re-enters the kernel.
    let data = process.data.get().as_mut().unwrap();
    let trap_frame = data.trap_frame.as_mut().unwrap();
    trap_frame.kernel_satp = read_satp() as u64;
    trap_frame.kernel_sp = (data.kernel_stack + PAGE_SIZE) as u64;
    trap_frame.kernel_trap = usertrap as u64;
    trap_frame.kernel_hartid = read_tp() as u64;

    // set up the registers that trampoline.S's sret will use
    // to get to user space.

    // set S Previous Privilege mode to User.
    let mut x = read_sstatus();
    x &= !SSTATUS_SPP;
    x |= SSTATUS_SPIE;
    write_sstatus(x);

    // set S Exception Program Counter to the saved user pc.
    write_sepc(trap_frame.epc as usize);

    // tell trampoline.S the user page table to switch to.
    let satp = make_satp(data.page_table.as_ref().unwrap());

    // jump to trampoline.S at the top of memory, which
    // switches to the user page table, restores user registers,
    // and switches to user mode with sret.
    let func = TRAMPOLINE + (userret - trampoline);
    let func: extern "C" fn(usize, usize) = transmute(func);
    func(TRAPFRAME, satp);
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