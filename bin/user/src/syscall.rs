
pub fn fork() -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 1"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}

pub fn exit(_code: isize) -> ! {
    unsafe {
        llvm_asm!("li a7, 2"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        loop {}
    }
}

pub fn wait(_addr: *const usize) -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 3"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}

pub fn read(_fd: usize, _addr: usize, _size: usize) -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 5"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}

pub fn exec(_path: *const u8, _argv: usize) -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 7"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}

pub fn fstat(_fd: usize, _addr: usize) -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 8"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}

pub fn dup(_fd: usize) -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 10"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}

pub fn sbrk(_size: usize) -> *const u8 {
    unsafe {
        let mut x: *const u8;
        llvm_asm!("li a7, 12"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}

pub fn open(_path: *const u8, _mode: usize) -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 15"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}

pub fn write(_fd: usize, _str: *const u8, _size: usize) -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 16"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}

pub fn mknod(_path: *const u8, _major: usize, _minor: usize) -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 17"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}

pub fn close(_fd: usize) -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 21"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}