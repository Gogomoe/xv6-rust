use alloc::string::String;
use alloc::vec::Vec;
use core::intrinsics::size_of;

use cstr_core::CString;

use param_lib::MAX_ARG;

use crate::file_system::elf::{ELF_MAGIC, ELF_PROG_LOAD, ElfHeader, ProgramHeader};
use crate::file_system::inode::{ICACHE, INode};
use crate::file_system::LOG;
use crate::file_system::path::find_inode;
use crate::memory::{ActivePageTable, copy_in, copy_in_string, copy_out, page_round_up, PAGE_SIZE};
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
    let load_result = load_program(&path);
    if load_result.is_none() {
        return u64::max_value();
    }

    let (page_table, size, elf_header) = load_result.unwrap();
    let result = prepare_process(path, argv, page_table, size, elf_header);

    return if result.is_none() {
        u64::max_value()
    } else {
        result.unwrap() as u64
    };
}

fn load_program(path: &String) -> Option<(ActivePageTable, usize, ElfHeader)> {
    let log = unsafe { &mut LOG };

    log.begin_op();

    let ip = find_inode(path);
    if ip.is_none() {
        log.end_op();
        return None;
    }
    let ip = ip.unwrap();
    let guard = ip.lock();

    // Check ELF header
    let mut elf_header = ElfHeader::new();
    if !check_elf_header(&mut elf_header, ip) {
        ip.unlock(guard);
        ICACHE.put(ip);
        log.end_op();
        return None;
    }

    let page_table = user_virtual_memory::alloc_page_table(CPU_MANAGER.my_proc().data().trap_frame);
    if page_table.is_none() {
        ip.unlock(guard);
        ICACHE.put(ip);
        log.end_op();
        return None;
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
            return None;
        }
    };

    ip.unlock(guard);
    ICACHE.put(ip);
    log.end_op();

    return Some((page_table, size, elf_header));
}

fn prepare_process(path: String, argv: Vec<String>, mut page_table: ActivePageTable, mut size: usize, elf_header: ElfHeader) -> Option<usize> {
    let process = CPU_MANAGER.my_proc();
    let old_size = process.data().size;

    // Allocate two pages at the next page boundary.
    // Use the second as the user stack.
    size = page_round_up(size);
    size = match user_virtual_memory::alloc_user_virtual_memory(&mut page_table, size, size + 2 * PAGE_SIZE) {
        None => {
            user_virtual_memory::free_page_table(page_table, size);
            return None;
        }
        Some(new_size) => {
            new_size
        }
    };
    let stack_top = size;
    let stack_base = stack_top - PAGE_SIZE;

    user_virtual_memory::make_guard_page(&mut page_table, stack_top - 2 * PAGE_SIZE);

    // Push argument strings, prepare rest of stack in ustack.
    if argv.len() >= MAX_ARG {
        user_virtual_memory::free_page_table(page_table, size);
        return None;
    }
    let mut sp = stack_top;
    let mut user_stack = Vec::new();
    for argc in 0..argv.len() {
        let c_str = CString::new(argv[argc].clone()).expect("CString::new failed");
        let c_bytes = c_str.to_bytes_with_nul();

        sp -= c_bytes.len();
        sp -= sp % 16; // riscv sp must be 16-byte aligned

        if sp < stack_base {
            user_virtual_memory::free_page_table(page_table, size);
            return None;
        }
        let copy_result = unsafe { copy_out(&page_table, sp, c_bytes.as_ptr() as usize, c_bytes.len()) };
        if !copy_result {
            user_virtual_memory::free_page_table(page_table, size);
            return None;
        }

        user_stack.push(sp);
    }
    user_stack.push(0);

    // push the array of argv[] pointers.
    sp -= (argv.len() + 1) * size_of::<u64>();
    sp -= sp % 16;
    if sp < stack_base {
        user_virtual_memory::free_page_table(page_table, size);
        return None;
    }
    let copy_result = unsafe { copy_out(&page_table, sp, user_stack.as_ptr() as usize, (argv.len() + 1) * size_of::<u64>()) };
    if !copy_result {
        user_virtual_memory::free_page_table(page_table, size);
        return None;
    }

    // arguments to user main(argc, argv)
    // argc is returned via the system call return
    // value, which goes in a0.
    let data = process.data();
    let trap_frame = unsafe { data.trap_frame.as_mut() }.unwrap();

    trap_frame.a1 = sp as u64;

    // Save program name for debugging.
    let last = path.rfind("/").map_or(0, |it| it + 1);
    let (_, filename) = path.split_at(last);
    data.name = String::from(filename);

    // Commit to the user image.
    let old_page_table = data.page_table.take().unwrap();
    data.page_table = Some(page_table);
    data.size = size;
    trap_frame.epc = elf_header.entry;  // initial program counter = main
    trap_frame.sp = sp as u64; // initial stack pointer

    user_virtual_memory::free_page_table(old_page_table, old_size);

    return Some(argv.len()); // this ends up in a0, the first argument to main(argc, argv)
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