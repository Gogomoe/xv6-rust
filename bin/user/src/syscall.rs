use cstr_core::CString;

pub use file_control_lib::{
    OPEN_CREATE, OPEN_READ_ONLY, OPEN_READ_WRITE, OPEN_TRUNC, OPEN_WRITE_ONLY,
};
pub use file_system_lib::FileStatus;

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

pub fn wait(_addr: *mut usize) -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 3"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}

pub fn pipe(_fdarray: *mut [usize; 2]) -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 6"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}

pub fn read(_fd: usize, _addr: *mut u8, _size: usize) -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 5"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}

pub fn kill(_pid: usize) -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 6"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}

fn _exec(_path: *const u8, _argv: *const [*const u8]) -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 7"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}

#[inline]
pub fn exec(_path: &str, _argv: *const [*const u8]) -> isize {
    let _path = CString::new(_path).expect("open syscall: CString::new failed");
    _exec(_path.as_ptr(), _argv)
}

pub fn fstat(_fd: usize, _addr: *mut FileStatus) -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 8"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}

pub fn chdir(_path: *const u8) -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 9"::::"volatile");
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

pub fn getpid() -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 11"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}

pub fn sbrk(_size: usize) -> *mut u8 {
    unsafe {
        let mut x: *mut u8;
        llvm_asm!("li a7, 12"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}

pub fn sleep(_ticks: usize) -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 13"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}

pub fn uptime() -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 14"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}

fn _open(_path: *const u8, _mode: usize) -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 15"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}

#[inline]
pub fn open(_path: &str, _mode: usize) -> isize {
    let _path = CString::new(_path).expect("open syscall: CString::new failed");
    _open(_path.as_ptr(), _mode)
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

fn _mknod(_path: *const u8, _major: usize, _minor: usize) -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 17"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}

#[inline]
pub fn mknod(_path: &str, _major: usize, _minor: usize) -> isize {
    let _path = CString::new(_path).expect("open syscall: CString::new failed");
    _mknod(_path.as_ptr(), _major, _minor)
}

pub fn unlink(_path: *const u8) -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 18"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}

pub fn link(_old: *const u8, _new: *const u8) -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 19"::::"volatile");
        llvm_asm!("ecall"::::"volatile");
        llvm_asm!("mv $0, a0":"=r"(x):::"volatile");
        return x;
    }
}

pub fn mkdir(_path: *const u8) -> isize {
    unsafe {
        let mut x: isize;
        llvm_asm!("li a7, 20"::::"volatile");
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
