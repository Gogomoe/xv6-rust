pub fn write(_fd: usize, _str: *const u8, _size: usize) -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 16"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}