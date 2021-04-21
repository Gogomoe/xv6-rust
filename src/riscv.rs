#![allow(dead_code)]

pub unsafe fn read_mhartid() -> usize {
    let mut x: usize;
    llvm_asm!("csrr $0, mhartid":"=r"(x):::"volatile");
    return x;
}

pub const MSTATUS_MPP_MASK: usize = 3 << 11;
pub const MSTATUS_MPP_M: usize = 3 << 11;
pub const MSTATUS_MPP_S: usize = 1 << 11;
pub const MSTATUS_MPP_U: usize = 0 << 11;

pub unsafe fn read_mstatus() -> usize {
    let mut x: usize;
    llvm_asm!("csrr $0, mstatus":"=r"(x):::"volatile");
    return x;
}

pub unsafe fn write_mstatus(x: usize) {
    llvm_asm!("csrw mstatus, $0"::"r"(x)::"volatile");
}

pub unsafe fn write_mepc(x: usize) {
    llvm_asm!("csrw mepc, $0"::"r"(x)::"volatile");
}

pub unsafe fn write_satp(x: usize) {
    llvm_asm!("csrw satp, $0"::"r"(x)::"volatile");
}

pub unsafe fn write_medeleg(x: usize) {
    llvm_asm!("csrw medeleg, $0"::"r"(x)::"volatile");
}

pub unsafe fn write_mideleg(x: usize) {
    llvm_asm!("csrw mideleg, $0"::"r"(x)::"volatile");
}

pub unsafe fn read_sie() -> usize {
    let mut x: usize;
    llvm_asm!("csrr $0, sie":"=r"(x):::"volatile");
    return x;
}

pub unsafe fn write_sie(x: usize) {
    llvm_asm!("csrw sie, $0"::"r"(x)::"volatile");
}

pub const SIE_SEIE: usize = 1 << 9;  /* external */
pub const SIE_STIE: usize = 1 << 5;  /* timer */
pub const SIE_SSIE: usize = 1 << 1;  /* software */

pub unsafe fn read_tp() -> usize {
    let mut x: usize;
    llvm_asm!("mv $0, tp":"=r"(x):::"volatile");
    return x;
}

pub unsafe fn write_tp(x: usize) {
    llvm_asm!("mv tp, $0"::"r"(x)::"volatile");
}