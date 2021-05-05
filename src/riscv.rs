#![allow(dead_code)]

// which hart (core) is this?
#[inline]
pub unsafe fn read_mhartid() -> usize {
    let mut x: usize;
    llvm_asm!("csrr $0, mhartid":"=r"(x):::"volatile");
    return x;
}

// Machine Status Register, mstatus

pub const MSTATUS_MPP_MASK: usize = 3 << 11; /* previous mode. */
pub const MSTATUS_MPP_M: usize = 3 << 11;
pub const MSTATUS_MPP_S: usize = 1 << 11;
pub const MSTATUS_MPP_U: usize = 0 << 11;
pub const MSTATUS_MIE: usize = 1 << 3; /* machine-mode interrupt enable. */

#[inline]
pub unsafe fn read_mstatus() -> usize {
    let mut x: usize;
    llvm_asm!("csrr $0, mstatus":"=r"(x):::"volatile");
    return x;
}

#[inline]
pub unsafe fn write_mstatus(x: usize) {
    llvm_asm!("csrw mstatus, $0"::"r"(x)::"volatile");
}

// machine exception program counter, holds the
// instruction address to which a return from
// exception will go.
#[inline]
pub unsafe fn write_mepc(x: usize) {
    llvm_asm!("csrw mepc, $0"::"r"(x)::"volatile");
}

// Supervisor Status Register, sstatus

pub const SSTATUS_SPP: usize = 1 << 8;   /* Previous mode, 1=Supervisor, 0=User */
pub const SSTATUS_SPIE: usize = 1 << 5;  /* Supervisor Previous Interrupt Enable */
pub const SSTATUS_UPIE: usize = 1 << 4;  /* User Previous Interrupt Enable */
pub const SSTATUS_SIE: usize = 1 << 1;   /* Supervisor Interrupt Enable */
pub const SSTATUS_UIE: usize = 1 << 0;   /* User Interrupt Enable */

#[inline]
pub unsafe fn read_sstatus() -> usize {
    let mut x: usize;
    llvm_asm!("csrr $0, sstatus":"=r"(x):::"volatile");
    return x;
}

#[inline]
pub unsafe fn write_sstatus(x: usize) {
    llvm_asm!("csrw sstatus, $0"::"r"(x)::"volatile");
}

// Supervisor Interrupt Pending
#[inline]
pub unsafe fn read_sip() -> usize {
    let mut x: usize;
    llvm_asm!("csrr $0, sip":"=r"(x):::"volatile");
    return x;
}

#[inline]
pub unsafe fn write_sip(x: usize) {
    llvm_asm!("csrw sip, $0"::"r"(x)::"volatile");
}

// Supervisor Interrupt Enable
pub const SIE_SEIE: usize = 1 << 9;  /* external */
pub const SIE_STIE: usize = 1 << 5;  /* timer */
pub const SIE_SSIE: usize = 1 << 1;  /* software */

#[inline]
pub unsafe fn read_sie() -> usize {
    let mut x: usize;
    llvm_asm!("csrr $0, sie":"=r"(x):::"volatile");
    return x;
}

#[inline]
pub unsafe fn write_sie(x: usize) {
    llvm_asm!("csrw sie, $0"::"r"(x)::"volatile");
}

// Machine-mode Interrupt Enable
pub const MIE_MEIE: usize = 1 << 11; /* external */
pub const MIE_MTIE: usize = 1 << 7;  /* timer */
pub const MIE_MSIE: usize = 1 << 3;  /* software */

#[inline]
pub unsafe fn read_mie() -> usize {
    let mut x: usize;
    llvm_asm!("csrr $0, mie":"=r"(x):::"volatile");
    return x;
}

#[inline]
pub unsafe fn write_mie(x: usize) {
    llvm_asm!("csrw mie, $0"::"r"(x)::"volatile");
}

// machine exception program counter, holds the
// instruction address to which a return from
// exception will go.
#[inline]
pub unsafe fn write_sepc(x: usize) {
    llvm_asm!("csrw sepc, $0"::"r"(x)::"volatile");
}

#[inline]
pub unsafe fn read_sepc() -> usize {
    let mut x: usize;
    llvm_asm!("csrr $0, sepc":"=r"(x):::"volatile");
    return x;
}

// Machine Exception Delegation
#[inline]
pub unsafe fn read_medeleg() -> usize {
    let mut x: usize;
    llvm_asm!("csrr $0, medeleg":"=r"(x):::"volatile");
    return x;
}

#[inline]
pub unsafe fn write_medeleg(x: usize) {
    llvm_asm!("csrw medeleg, $0"::"r"(x)::"volatile");
}

// Machine Interrupt Delegation
#[inline]
pub unsafe fn read_mideleg() -> usize {
    let mut x: usize;
    llvm_asm!("csrr $0, mideleg":"=r"(x):::"volatile");
    return x;
}

#[inline]
pub unsafe fn write_mideleg(x: usize) {
    llvm_asm!("csrw mideleg, $0"::"r"(x)::"volatile");
}

// Supervisor Trap-Vector Base Address
// low two bits are mode.
#[inline]
pub unsafe fn write_stvec(x: usize) {
    llvm_asm!("csrw stvec, $0"::"r"(x)::"volatile");
}

#[inline]
pub unsafe fn read_stvec() -> usize {
    let mut x: usize;
    llvm_asm!("csrr $0, stvec":"=r"(x):::"volatile");
    return x;
}

// Machine-mode interrupt vector
#[inline]
pub unsafe fn write_mtvec(x: usize) {
    llvm_asm!("csrw mtvec, $0"::"r"(x)::"volatile");
}

// supervisor address translation and protection;
// holds the address of the page table.
#[inline]
pub unsafe fn write_satp(x: usize) {
    llvm_asm!("csrw satp, $0"::"r"(x)::"volatile");
}

#[inline]
pub unsafe fn read_satp() -> usize {
    let mut x: usize;
    llvm_asm!("csrr $0, satp":"=r"(x):::"volatile");
    return x;
}

// Supervisor Scratch register, for early trap handler in trampoline.S.
#[inline]
pub unsafe fn write_sscratch(x: usize) {
    llvm_asm!("csrw sscratch, $0"::"r"(x)::"volatile");
}

#[inline]
pub unsafe fn write_mscratch(x: usize) {
    llvm_asm!("csrw mscratch, $0"::"r"(x)::"volatile");
}

// Supervisor Trap Cause
#[inline]
pub unsafe fn read_scause() -> usize {
    let mut x: usize;
    llvm_asm!("csrr $0, scause":"=r"(x):::"volatile");
    return x;
}

// Supervisor Trap Value
#[inline]
pub unsafe fn read_stval() -> usize {
    let mut x: usize;
    llvm_asm!("csrr $0, stval":"=r"(x):::"volatile");
    return x;
}

// Machine-mode Counter-Enable
#[inline]
pub unsafe fn write_mcounteren(x: usize) {
    llvm_asm!("csrw mcounteren, $0"::"r"(x)::"volatile");
}

#[inline]
pub unsafe fn read_mcounteren() -> usize {
    let mut x: usize;
    llvm_asm!("csrr $0, mcounteren":"=r"(x):::"volatile");
    return x;
}

// machine-mode cycle counter
#[inline]
pub unsafe fn read_time() -> usize {
    let mut x: usize;
    llvm_asm!("csrr $0, time":"=r"(x):::"volatile");
    return x;
}

// enable device interrupts
#[inline]
pub unsafe fn intr_on() {
    write_sstatus(read_sstatus() | SSTATUS_SIE);
}

// disable device interrupts
#[inline]
pub unsafe fn intr_off() {
    write_sstatus(read_sstatus() & !SSTATUS_SIE);
}

// are device interrupts enabled?
#[inline]
pub unsafe fn intr_get() -> bool {
    let x = read_sstatus();
    return (x & SSTATUS_SIE) != 0;
}

#[inline]
pub unsafe fn read_sp() -> usize {
    let mut x: usize;
    llvm_asm!("mv $0, sp":"=r"(x):::"volatile");
    return x;
}

// read and write tp, the thread pointer, which holds
// this core's hartid (core number), the index into cpus[].
#[inline]
pub unsafe fn read_tp() -> usize {
    let mut x: usize;
    llvm_asm!("mv $0, tp":"=r"(x):::"volatile");
    return x;
}

#[inline]
pub unsafe fn write_tp(x: usize) {
    llvm_asm!("mv tp, $0"::"r"(x)::"volatile");
}

#[inline]
pub unsafe fn read_ra() -> usize {
    let mut x: usize;
    llvm_asm!("mv $0, ra":"=r"(x):::"volatile");
    return x;
}

// flush the TLB.
#[inline]
pub unsafe fn sfence_vma() {
    // the zero, zero means flush all TLB entries.
    llvm_asm!("sfence.vma zero, zero"::::"volatile");
}