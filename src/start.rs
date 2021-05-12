use core::sync::atomic::{AtomicBool, Ordering};

use crate::driver::DISK;
use crate::memory::layout::CLINT;
use crate::memory::PHYSICAL_MEMORY;
use crate::param::MAX_CPU_NUMBER;
use crate::process::PROCESS_MANAGER;

#[no_mangle]
pub unsafe fn start() -> ! {
    use crate::riscv::*;
    // set M Previous Privilege mode to Supervisor, for mret.
    let mut x = read_mstatus();
    x &= !MSTATUS_MPP_MASK;
    x |= MSTATUS_MPP_S;
    write_mstatus(x);

    // set M Exception Program Counter to main, for mret.
    // requires gcc -mcmodel=medany
    write_mepc(main as usize);

    // disable paging for now.
    write_satp(0);

    // delegate all interrupts and exceptions to supervisor mode.
    write_medeleg(0xffff);
    write_mideleg(0xffff);
    write_sie(read_sie() | SIE_SEIE | SIE_STIE | SIE_SSIE);

    // ask for clock interrupts.
    timer_init();

    // keep each CPU's hartid in its tp register, for cpuid().
    let id = read_mhartid();
    write_tp(id);

    // switch to supervisor mode and jump to main().
    llvm_asm!("mret"::::"volatile");

    loop {}
}

// scratch area for timer interrupt, one per CPU.
static mut MSCRATCH0: [usize; MAX_CPU_NUMBER * 32] = [0; MAX_CPU_NUMBER * 32];

// set up to receive timer interrupts in machine mode,
// which arrive at timervec in kernelvec.S,
// which turns them into software interrupts for
// devintr() in trap.c.
pub unsafe fn timer_init() {
    extern {
        fn timervec();
    }
    use crate::riscv::*;

    // each CPU has a separate source of timer interrupts.
    let id = read_mhartid();

    // ask the CLINT for a timer interrupt.
    let interval = 1000000; // cycles; about 1/10th second in qemu.
    let clint_mtimecmp: usize = CLINT + 0x4000 + 8 * id;
    let clint_mtime = CLINT + 0xBFF8;
    *(clint_mtimecmp as *mut usize) = *(clint_mtime as *const usize) + interval;

    // prepare information in scratch[] for timervec.
    // scratch[0..3] : space for timervec to save registers.
    // scratch[4] : address of CLINT MTIMECMP register.
    // scratch[5] : desired interval (in cycles) between timer interrupts.
    let scratch: *mut usize = &mut MSCRATCH0[32 * id] as *mut usize;
    *scratch.offset(4) = clint_mtimecmp;
    *scratch.offset(5) = interval;
    write_mscratch(scratch as usize);

    // set the machine-mode trap handler.
    write_mtvec(timervec as usize);

    // enable machine-mode interrupts.
    write_mstatus(read_mstatus() | MSTATUS_MIE);

    // enable machine-mode timer interrupts.
    write_mie(read_mie() | MIE_MTIE);
}

pub unsafe fn main() -> ! {
    static STARTED: AtomicBool = AtomicBool::new(false);

    let cpuid = crate::riscv::read_tp();
    if cpuid == 0 {
        crate::console::uart::uart_init();
        println!("xv6 kernel is booting");
        PHYSICAL_MEMORY.init();
        crate::memory::virtual_memory::virtual_memory_init();
        crate::memory::kernel_virtual_memory::kernel_page_table_init();
        crate::memory::kernel_virtual_memory::hart_init(); // turn on paging
        crate::memory::kernel_heap::kernel_heap_init();
        PROCESS_MANAGER.init();
        crate::trap::trap_hart_init();
        crate::plic::plic_init();
        crate::plic::plic_hart_init();
        DISK.init();
        PROCESS_MANAGER.user_init();
        crate::syscall::system_call_init();

        STARTED.store(true, Ordering::SeqCst);
        println!("xv6 kernel boots successfully");
    } else {
        while !STARTED.load(Ordering::SeqCst) {}

        println!("hart {} starting", cpuid);
        crate::memory::kernel_virtual_memory::hart_init(); // turn on paging
        crate::trap::trap_hart_init();
        crate::plic::plic_hart_init();
    }

    PROCESS_MANAGER.scheduler();
}