use alloc::string::String;
use core::ptr::null_mut;

use file_control_lib::{OPEN_CREATE, OPEN_READ_ONLY, OPEN_READ_WRITE, OPEN_TRUNC, OPEN_WRITE_ONLY};
use file_system_lib::{TYPE_DEVICE, TYPE_DIR, TYPE_FILE};
use param_lib::MAX_DEV_NUMBER;

use crate::file_system::{FILE_TABLE, LOG};
use crate::file_system::file::FileType::{DEVICE, INODE};
use crate::file_system::inode::{ICACHE, INode};
use crate::file_system::path::{find_inode, find_inode_parent};
use crate::process::CPU_MANAGER;
use crate::sleep_lock::SleepLockGuard;
use crate::syscall::{read_arg_string, read_arg_usize};

fn fd_alloc() -> Option<usize> {
    let process = CPU_MANAGER.my_proc();
    let open_files = process.data().open_file;
    for i in 0..open_files.len() {
        if open_files[i].is_null() {
            return Some(i);
        }
    }
    return None;
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

    let fd = fd_alloc();
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