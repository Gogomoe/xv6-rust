extern crate file_system_lib;
extern crate param_lib;

use file_system_lib::{
    iblock, Dirent, INodeDisk, SuperBlock, BLOCK_SIZE, DIRECT_COUNT, INDIRECT_COUNT,
    DIRECTORY_SIZE, FSMAGIC, IPB, MAX_FILE_COUNT, ROOT_INO, TYPE_DIR, TYPE_FILE,
};
use lazy_static::lazy_static;
use param_lib::{FILE_SYSTEM_SIZE, LOG_SIZE};
use std::env;
use std::ffi::CString;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::mem;
use std::process;
use std::ptr;
use std::slice;
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Mutex,
};

const NINODES: u32 = 200;
#[allow(non_upper_case_globals)]
const nbitmap: u32 = FILE_SYSTEM_SIZE / (BLOCK_SIZE as u32 * 8) + 1;
#[allow(non_upper_case_globals)]
const ninodeblocks: u32 = NINODES / IPB + 1;
#[allow(non_upper_case_globals)]
const nlog: u32 = LOG_SIZE as u32;

#[allow(non_upper_case_globals)]
const nmeta: u32 = 2 + nlog + ninodeblocks + nbitmap;
#[allow(non_upper_case_globals)]
const nblocks: u32 = FILE_SYSTEM_SIZE - nmeta;

#[allow(non_upper_case_globals)]
static freeinode: AtomicU32 = AtomicU32::new(1);
#[allow(non_upper_case_globals)]
static freeblock: AtomicU32 = AtomicU32::new(nmeta);

lazy_static! {
    static ref SUPERBLOCK: Mutex<SuperBlock> = Mutex::new(SuperBlock {
        magic: FSMAGIC,
        size: xint(FILE_SYSTEM_SIZE),
        blocks_number: xint(nblocks),
        inode_number: xint(NINODES),
        log_number: xint(nlog),
        log_start: xint(2),
        inode_start: xint(2 + nlog),
        block_map_start: xint(2 + nlog + ninodeblocks)
    });
    static ref ARGS: Vec<String> = {
        if env::args().len() < 2 {
            eprintln!("Usage: mkfs fs.img files...");
            process::exit(1);
        }
        env::args().collect()
    };
    static ref FSFD: Mutex<File> = {
        assert_eq!(0, BLOCK_SIZE % mem::size_of::<INodeDisk>());
        assert_eq!(0, BLOCK_SIZE % mem::size_of::<Dirent>());
        Mutex::new(
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&ARGS[1])
                .expect(&ARGS[1]),
        )
    };
}

fn xshort(x: u16) -> u16 {
    let mut y = 0;
    let a: *mut u8 = &mut y as *mut u16 as *mut u8;
    unsafe {
        *a = x as u8;
        *(a.add(1)) = (x >> 8) as u8;
    }
    y
}

fn xint(x: u32) -> u32 {
    let mut y = 0;
    let a: *mut u8 = &mut y as *mut u32 as *mut u8;
    unsafe {
        *a = x as u8;
        *(a.add(1)) = (x >> 8) as u8;
        *(a.add(2)) = (x >> 16) as u8;
        *(a.add(3)) = (x >> 24) as u8;
    }
    y
}

fn main() {
    println!("nmeta {} (boot, super, log blocks {} inode blocks {}, bitmap blocks {}) blocks {} total {}",
            nmeta, nlog, ninodeblocks, nbitmap, nblocks, FILE_SYSTEM_SIZE);
    let zeroes = [0u8; BLOCK_SIZE];
    for i in 0..FILE_SYSTEM_SIZE {
        wsect(i, &zeroes);
    }

    let mut buf = [0u8; BLOCK_SIZE];
    unsafe {
        ptr::copy(
            &*SUPERBLOCK.lock().unwrap() as *const SuperBlock,
            buf.as_mut_ptr() as *mut SuperBlock,
            1,
        );
    }
    wsect(1, &buf);

    let rootino = ialloc(TYPE_DIR);
    assert_eq!(rootino, ROOT_INO);

    let mut de = Dirent {
        inum: xshort(rootino as u16),
        name: {
            let mut name = [0u8; DIRECTORY_SIZE];
            let tmp = CString::new(".").unwrap();
            let ttmp = tmp.as_bytes_with_nul();
            let (left, _) = name.split_at_mut(ttmp.len());
            left.clone_from_slice(&ttmp);
            name
        },
    };
    iappend(rootino, &mut de, mem::size_of::<Dirent>());

    de = Dirent {
        inum: xshort(rootino as u16),
        name: {
            let mut name = [0u8; DIRECTORY_SIZE];
            let tmp = CString::new("..").unwrap();
            let ttmp = tmp.as_bytes_with_nul();
            let (left, _) = name.split_at_mut(ttmp.len());
            left.clone_from_slice(&ttmp);
            name
        },
    };
    iappend(rootino, &mut de, mem::size_of::<Dirent>());

    for i in 2..ARGS.len() {
        let mut filename = String::from("target/riscv64gc-unknown-none-elf/debug/");
        filename.push_str(&ARGS[i]);
        if let Ok(mut fd) = File::open(filename) {
            let inum = ialloc(TYPE_FILE);

            de = Dirent {
                inum: xshort(inum as u16),
                name: {
                    let mut name = [0u8; DIRECTORY_SIZE];
                    let tmp = CString::new(&ARGS[i] as &str).unwrap();
                    let ttmp = tmp.as_bytes_with_nul();
                    let (left, _) = name.split_at_mut(ttmp.len());
                    left.clone_from_slice(&ttmp);
                    name
                },
            };
            iappend(rootino, &mut de, mem::size_of::<Dirent>());

            while let Ok(i) = fd.read(&mut buf) {
                if i == 0 {
                    break;
                }
                iappend(inum, &mut buf, i);
            }
        } else {
            eprintln!("{}", ARGS[i]);
            process::exit(1);
        }
    }

    let mut din = INodeDisk::new();
    rinode(rootino, &mut din);
    let mut off = xint(din.size);
    off = ((off / BLOCK_SIZE as u32) + 1) * BLOCK_SIZE as u32;
    din.size = xint(off);
    winode(rootino, &din);

    balloc(freeblock.load(Ordering::Relaxed) as usize);

    process::exit(0);
}

fn wsect<T>(sec: u32, buf: &[T]) {
    let mut fd = FSFD.lock().unwrap();
    fd.seek(SeekFrom::Start(sec as u64 * BLOCK_SIZE as u64))
        .expect("lseek");
    fd.write(unsafe {
        slice::from_raw_parts(
            buf as *const _ as *const u8,
            buf.len() * mem::size_of::<T>(),
        )
    })
    .expect("write");
}

fn winode(inum: u32, ip: &INodeDisk) {
    let mut buf = [0u8; BLOCK_SIZE];
    let bn = iblock(inum, &*SUPERBLOCK.lock().unwrap());
    rsect(bn, &mut buf);
    unsafe {
        let dip = (buf.as_mut_ptr() as *mut INodeDisk).add((inum % IPB) as usize);
        ptr::copy(ip, dip, 1);
    }
    wsect(bn, &buf);
}

fn rinode(inum: u32, ip: &mut INodeDisk) {
    let mut buf = [0u8; BLOCK_SIZE];
    let bn = iblock(inum, &*SUPERBLOCK.lock().unwrap());
    rsect(bn, &mut buf);
    unsafe {
        let dip = (buf.as_ptr() as *const INodeDisk).add((inum % IPB) as usize);
        ptr::copy(dip, ip, 1);
    }
}

fn rsect<T>(sec: u32, buf: &mut [T]) {
    let mut fd = FSFD.lock().unwrap();
    fd.seek(SeekFrom::Start(sec as u64 * BLOCK_SIZE as u64))
        .expect("lseek");
    fd.read(unsafe {
        slice::from_raw_parts_mut(buf as *mut _ as *mut u8, buf.len() * mem::size_of::<T>())
    })
    .expect("read");
}

fn ialloc(t: u16) -> u32 {
    let inum: u32 = freeinode.load(Ordering::Relaxed);
    freeinode.store(inum + 1, Ordering::Relaxed);

    let mut din = INodeDisk::new();
    din.types = xshort(t);
    din.nlink = xshort(1);
    din.size = xint(0);
    winode(inum, &din);
    inum
}

fn balloc(used: usize) {
    println!("balloc: first {} blocks have been allocated", used);
    assert_eq!(true, used < BLOCK_SIZE * 8);
    let mut buf = [0u8; BLOCK_SIZE];
    for i in 0..used {
        buf[i / 8] = buf[i / 8] | (0x1 << (i % 8));
    }
    println!(
        "balloc: write bitmap block at sector {}",
        SUPERBLOCK.lock().unwrap().block_map_start
    );
    wsect(SUPERBLOCK.lock().unwrap().block_map_start, &buf);
}

fn iappend<T>(inum: u32, xp: &mut T, mut n: usize) {
    let mut din = INodeDisk::new();
    rinode(inum, &mut din);

    let mut buf = [0u8; BLOCK_SIZE];
    let mut p = xp as *const T;
    let mut off = xint(din.size) as usize;
    println!("append inum {} at off {} sz {}", inum, off, n);
    while n > 0 {
        let x;
        let fbn = off / BLOCK_SIZE;
        assert_eq!(true, fbn < MAX_FILE_COUNT);
        if fbn < DIRECT_COUNT {
            if xint(din.addr[fbn]) == 0 {
                din.addr[fbn] = xint(freeblock.load(Ordering::Relaxed));
                freeblock.store(freeblock.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
            }
            x = xint(din.addr[fbn]);
        } else {
            if xint(din.addr[DIRECT_COUNT]) == 0 {
                din.addr[DIRECT_COUNT] = xint(freeblock.load(Ordering::Relaxed));
                freeblock.store(freeblock.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
            }
            let mut indirect = [0u32; INDIRECT_COUNT];
            rsect(xint(din.addr[DIRECT_COUNT]), &mut indirect);
            if indirect[fbn - DIRECT_COUNT] == 0 {
                indirect[fbn - DIRECT_COUNT] = xint(freeblock.load(Ordering::Relaxed));
                freeblock.store(freeblock.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
                wsect(xint(din.addr[DIRECT_COUNT]), &indirect);
            }
            x = xint(indirect[fbn - DIRECT_COUNT]);
        }
        let n1 = n.min((fbn + 1) * BLOCK_SIZE - off);
        rsect(x, &mut buf);
        unsafe {
            ptr::copy(
                p as *const u8,
                (buf.as_mut_ptr() as usize + off - (fbn * BLOCK_SIZE)) as *mut T as *mut u8,
                n1,
            );
        }
        wsect(x, &buf);
        n -= n1;
        off += n1;
        p = (p as usize + n1) as *const T;
    }
    din.size = xint(off as u32);
    winode(inum, &din);
}
