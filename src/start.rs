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

pub unsafe fn timer_init() {}

pub unsafe fn main() -> ! {
    let cpuid = crate::riscv::read_tp();
    if cpuid == 0 {
        crate::console::uart::uart_init();
        println!("xv6 kernel is booting");
    }

    loop {}
}