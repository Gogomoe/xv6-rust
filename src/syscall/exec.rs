use alloc::string::String;
use alloc::vec::Vec;
use core::intrinsics::size_of;

use crate::file_system::elf::{ELF_MAGIC, ElfHeader, ProgramHeader, ELF_PROG_LOAD};
use crate::file_system::inode::{ICACHE, INode};
use crate::file_system::LOG;
use crate::file_system::path::find_inode;
use crate::memory::{copy_in, copy_in_string, ActivePageTable, PAGE_SIZE};
use crate::memory::user_virtual_memory;
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
    let mut elf_header = ElfHeader::new();
    if !check_elf_header(&mut elf_header, ip) {
        ip.unlock(guard);
        ICACHE.put(ip);
        log.end_op();
        return u64::max_value();
    }

    let page_table = user_virtual_memory::alloc_page_table(CPU_MANAGER.my_proc().data().trap_frame);
    if page_table.is_none() {
        ip.unlock(guard);
        ICACHE.put(ip);
        log.end_op();
        return u64::max_value();
    }
    let mut page_table = page_table.unwrap();

    let load_result = load_program_into_memory(&mut page_table, &elf_header, ip);
    let size = match load_result {
        Ok(sz) => { sz }
        Err(sz) => {
            user_virtual_memory::free_page_table(page_table, sz);
            ip.unlock(guard);
            ICACHE.put(ip);
            log.end_op();
            return u64::max_value();
        }
    };

    ip.unlock(guard);
    ICACHE.put(ip);
    log.end_op();

    todo!();
}

fn check_elf_header(elf_header: &mut ElfHeader, ip: &INode) -> bool {
    let size_of_elf_header = size_of::<ElfHeader>() as u32;
    if ip.read(false, elf_header as *const _ as usize, 0, size_of_elf_header) != size_of_elf_header {
        return false;
    }
    if elf_header.magic != ELF_MAGIC {
        return false;
    }
    return true;
}

fn load_program_into_memory(page_table: &mut ActivePageTable, elf_header: &ElfHeader, ip: &INode) -> Result<usize, usize> {
    // Load program into memory.
    let mut off = elf_header.phoff as u32;
    let size_of_program_header = size_of::<ProgramHeader>() as u32;

    let mut size = 0;
    for _ in 0..(elf_header.phnum as usize) {
        let ph = ProgramHeader::new();
        if ip.read(false, &ph as *const _ as usize, off, size_of_program_header) != size_of_program_header {
            return Err(size);
        }
        if ph.types != ELF_PROG_LOAD {
            off += size_of_program_header;
            continue;
        }
        if ph.memsz < ph.filesz {
            return Err(size);
        }
        if ph.vaddr + ph.memsz < ph.vaddr {
            return Err(size);
        }
        let alloc_result = user_virtual_memory::alloc_user_virtual_memory(page_table, size, (ph.vaddr + ph.memsz) as usize);
        match alloc_result {
            Some(sz) => { size = sz }
            None => {
                return Err(size);
            }
        }
        if ph.vaddr as usize % PAGE_SIZE != 0 {
            return Err(size);
        }
        if !load_segement(page_table, ph.vaddr as usize, ip, ph.off as usize, ph.filesz as usize) {
            return Err(size);
        }
        off += size_of_program_header;
    }
    Ok(size)
}

// Load a program segment into pagetable at virtual address va.
// va must be page-aligned
// and the pages from va to va+sz must already be mapped.
// Returns 0 on success, -1 on failure.
fn load_segement(page_table: &mut ActivePageTable, va: usize, ip: &INode, offset: usize, size: usize) -> bool {
    assert_eq!(va % PAGE_SIZE, 0);

    for i in (0..size).step_by(PAGE_SIZE) {
        let pa = page_table.translate(va + i).unwrap();

        let n = if size - i < PAGE_SIZE {
            size - i
        } else {
            PAGE_SIZE
        } as u32;

        if ip.read(false, pa, (offset + i) as u32, n) != n {
            return false;
        }
    }
    return true;
}