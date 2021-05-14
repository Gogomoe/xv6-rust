use alloc::string::String;
use alloc::vec::Vec;
use core::intrinsics::size_of;

use crate::file_system::elf::{ElfHeader, ELF_MAGIC};
use crate::file_system::inode::{ICACHE};
use crate::file_system::LOG;
use crate::file_system::path::find_inode;
use crate::memory::{copy_in, copy_in_string};
use crate::process::CPU_MANAGER;
use crate::syscall::{read_arg_string, read_arg_usize};

pub fn sys_exec() -> u64 {
    let path = read_arg_string(0);
    if path.is_none() {
        return u64::max_value();
    }
    let argv = read_arg_string_array(1);
    if argv.is_none() {
        return u64::max_value();
    }

    return exec(path.unwrap(), argv.unwrap());
}

fn read_arg_string_array(pos: usize) -> Option<Vec<String>> {
    let page_table = CPU_MANAGER.my_proc().data().page_table.as_ref().unwrap();
    let mut array_addr = read_arg_usize(pos) as *const usize;

    let mut vec = Vec::new();
    let mut string_addr: usize = 0;

    copy_in(page_table, &mut string_addr as *mut usize as usize, array_addr as usize, size_of::<usize>());
    while string_addr != 0 {
        let string = copy_in_string(page_table, string_addr);
        if string.is_none() {
            return None;
        }
        vec.push(string.unwrap());

        array_addr = unsafe { array_addr.offset(1) };
        copy_in(page_table, &mut string_addr as *mut usize as usize, array_addr as usize, size_of::<usize>());
    }

    Some(vec)
}

fn exec(path: String, argv: Vec<String>) -> u64 {
    let log = unsafe { &mut LOG };

    log.begin_op();

    let ip = find_inode(&path);
    if ip.is_none() {
        log.end_op();
        return u64::max_value();
    }
    let ip = ip.unwrap();
    let guard = ip.lock();

    // Check ELF header
    let elf_header = ElfHeader::new();
    let size_of_elf_header = size_of::<ElfHeader>() as u32;
    if ip.read(false, &elf_header as *const _ as usize, 0, size_of_elf_header) != size_of_elf_header {
        ip.unlock(guard);
        ICACHE.put(ip);
        log.end_op();
        return u64::max_value();
    }
    if elf_header.magic != ELF_MAGIC {
        ip.unlock(guard);
        ICACHE.put(ip);
        log.end_op();
        return u64::max_value();
    }

    todo!();
}