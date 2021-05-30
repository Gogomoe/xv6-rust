use alloc::string::String;
use core::ptr::{null, null_mut};

use file_control_lib::{OPEN_CREATE, OPEN_READ_ONLY, OPEN_READ_WRITE, OPEN_TRUNC, OPEN_WRITE_ONLY};
use file_system_lib::{TYPE_DEVICE, TYPE_DIR, TYPE_FILE};
use param_lib::{MAX_DEV_NUMBER, MAX_OPEN_FILE_NUMBER};

use crate::file_system::{FILE_TABLE, LOG};
use crate::file_system::file::File;
use crate::file_system::file::FileType::{DEVICE, INODE};
use crate::file_system::inode::{ICACHE, INode};
use crate::file_system::path::{find_inode, find_inode_parent};
use crate::process::CPU_MANAGER;
use crate::sleep_lock::SleepLockGuard;
use crate::syscall::{read_arg_string, read_arg_usize};

// Fetch the nth word-sized system call argument as a file descriptor
// and return both the descriptor and the corresponding struct file.
fn read_arg_fd(pos: usize) -> Option<(usize, &'static File)> {
    let fd = read_arg_usize(pos);

    if fd >= MAX_OPEN_FILE_NUMBER {
        return None;
    }

    let file = CPU_MANAGER.my_proc().data().open_file[fd];
    if file.is_null() {
        return None;
    }

    Some((fd, unsafe { file.as_ref() }.unwrap()))
}

// Allocate a file descriptor for the given file.
// Takes over file reference from caller on success.
fn fd_alloc(file: &File) -> Option<usize> {
    let process = CPU_MANAGER.my_proc();
    let open_files = &mut process.data().open_file;
    for i in 0..open_files.len() {
        if open_files[i].is_null() {
            open_files[i] = file as *const File;
            return Some(i);
        }
    }
    return None;
}

pub fn sys_dup() -> u64 {
    let file = match read_arg_fd(0) {
        Some((_, file)) => { file }
        None => {
            return u64::max_value();
        }
    };
    let fd = match fd_alloc(file) {
        Some(fd) => { fd }
        None => {
            return u64::max_value();
        }
    };
    FILE_TABLE.dup(file);
    return fd as u64;
}

pub fn sys_read() -> u64 {
    let file = match read_arg_fd(0) {
        Some((_, file)) => { file }
        None => {
            return u64::max_value();
        }
    };
    let addr = read_arg_usize(1);
    let size = read_arg_usize(2);

    return FILE_TABLE.read(file, addr, size);
}

pub fn sys_write() -> u64 {
    let file = match read_arg_fd(0) {
        Some((_, file)) => { file }
        None => {
            return u64::max_value();
        }
    };
    let addr = read_arg_usize(1);
    let size = read_arg_usize(2);

    return FILE_TABLE.write(file, addr, size);
}

pub fn sys_close() -> u64 {
    let (fd, file) = match read_arg_fd(0) {
        Some(it) => { it }
        None => {
            return u64::max_value();
        }
    };
    CPU_MANAGER.my_proc().data().open_file[fd] = null();
    FILE_TABLE.close(file);

    return 0;
}

pub fn sys_fstat() -> u64 {
    let (_, file) = match read_arg_fd(0) {
        None => {
            return u64::max_value();
        }
        Some(it) => { it }
    };
    let addr = read_arg_usize(1);

    return if FILE_TABLE.stat(file, addr) {
        0
    } else {
        u64::max_value()
    };
}


fn create(path: &String, types: u16, major: u16, minor: u16) -> Option<(&'static INode, SleepLockGuard<()>)> {
    let dp = find_inode_parent(path);
    if dp.is_none() {
        return None;
    }
    let (dp, name) = dp.unwrap();

    let dp_guard = dp.lock();

    match dp.dir_lookup(&name, null_mut()) {
        Some(ip) => {
            dp.unlock_put(dp_guard);

            let guard = ip.lock();
            if types == TYPE_FILE && (ip.data().types == TYPE_FILE || ip.data().types == TYPE_DEVICE) {
                return Some((ip, guard));
            }
            ip.unlock_put(guard);
            return None;
        }
        _ => {}
    }

    let ip = ICACHE.alloc(dp.data().dev, types);
    let guard = ip.lock();
    ip.data().major = major;
    ip.data().minor = minor;
    ip.data().nlink = 1;
    ip.update();

    if types == TYPE_DIR { // Create . and .. entries.
        dp.data().nlink += 1; // for ".."
        dp.update();
        // No ip->nlink++ for ".": avoid cyclic ref count.
        if !ip.dir_link(&String::from("."), ip.data().inum) ||
            !ip.dir_link(&String::from(".."), ip.data().inum) {
            panic!("create dots");
        }
    }

    if !dp.dir_link(&name, ip.data().inum) {
        panic!("create: dirlink");
    }

    dp.unlock_put(dp_guard);

    return Some((ip, guard));
}

pub fn sys_open() -> u64 {
    let log = unsafe { &mut LOG };

    let path = read_arg_string(0);
    if path.is_none() {
        return u64::max_value();
    }
    let path = path.unwrap();
    let mode = read_arg_usize(1);

    log.begin_op();

    let (ip, guard) = if mode & OPEN_CREATE != 0 {
        let result = create(&path, TYPE_FILE, 0, 0);
        if result.is_none() {
            log.end_op();
            return u64::max_value();
        }
        result.unwrap()
    } else {
        let result = find_inode(&path);
        if result.is_none() {
            log.end_op();
            return u64::max_value();
        }
        let ip = result.unwrap();
        let guard = ip.lock();
        if ip.data().types == TYPE_DIR && mode != OPEN_READ_ONLY {
            ip.unlock_put(guard);
            log.end_op();
            return u64::max_value();
        }
        (ip, guard)
    };

    if ip.data().types == TYPE_DEVICE && ip.data().major >= MAX_DEV_NUMBER as u16 {
        ip.unlock_put(guard);
        log.end_op();
        return u64::max_value();
    }

    let file = FILE_TABLE.alloc();
    if file.is_none() {
        ip.unlock_put(guard);
        log.end_op();
        return u64::max_value();
    }
    let file = file.unwrap();

    let fd = fd_alloc(file);
    if fd.is_none() {
        FILE_TABLE.close(file);
        ip.unlock_put(guard);
        log.end_op();
        return u64::max_value();
    }
    let fd = fd.unwrap();

    if ip.data().types == TYPE_DEVICE {
        file.data().types = DEVICE;
        file.data().major = ip.data().major;
    } else {
        file.data().types = INODE;
        file.data().off = 0;
    }
    file.data().ip = Some(ip);
    file.data().readable = mode & OPEN_WRITE_ONLY == 0;
    file.data().writable = (mode & OPEN_WRITE_ONLY != 0) || (mode & OPEN_READ_WRITE != 0);

    if mode & OPEN_TRUNC != 0 && ip.data().types == TYPE_FILE {
        ip.truncate();
    }

    ip.unlock(guard);
    log.end_op();

    return fd as u64;
}

pub fn sys_mknod() -> u64 {
    let log = unsafe { &mut LOG };

    let path = read_arg_string(0);
    let major = read_arg_usize(1) as u16;
    let minor = read_arg_usize(2) as u16;

    if path.is_none() {
        return u64::max_value();
    }
    let path = path.unwrap();

    log.begin_op();

    let result = create(&path, TYPE_DEVICE, major, minor);
    if result.is_none() {
        log.end_op();
        return u64::max_value();
    }

    let (ip, guard) = result.unwrap();
    ip.unlock_put(guard);

    log.end_op();

    return 0;
}

pub fn sys_chdir() -> u64 {
    let log = unsafe { &mut LOG };

    let proc = CPU_MANAGER.my_proc().data();
    let path = read_arg_string(0);

    if path.is_none() {
        return u64::max_value();
    }
    let path = path.unwrap();

    log.begin_op();

    let ip = find_inode(&path);
    if ip.is_none() {
        log.end_op();
        return u64::max_value();
    }
    let ip = ip.unwrap();
    let guard = ip.lock();

    if ip.data().types != TYPE_DIR {
        ip.unlock_put(guard);
        log.end_op();
        return u64::max_value();
    }

    ip.unlock(guard);
    ICACHE.put(proc.current_dir.unwrap());
    log.end_op();
    proc.current_dir = Some(ip);

    return 0;
}