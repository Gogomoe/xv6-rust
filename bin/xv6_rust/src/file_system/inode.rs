use alloc::string::String;
use core::cell::UnsafeCell;
use core::cmp::min;
use core::intrinsics::size_of;
use core::ptr;
use core::ptr::null_mut;

use cstr_core::{c_char, CStr, CString};

use file_system_lib::{DIRECTORY_COUNT, DIRECTORY_INNER_COUNT, DIRECTORY_SIZE, Dirent, FileStatus, iblock, INodeDisk, IPB, MAX_FILE_COUNT, TYPE_DIR};
use param_lib::MAX_INODE_NUMBER;

use crate::file_system::{Block, BLOCK_CACHE, BLOCK_SIZE, LOG, SUPER_BLOCK};
use crate::memory::{either_copy_in, either_copy_out};
use crate::sleep_lock::{SleepLock, SleepLockGuard};
use crate::spin_lock::SpinLock;

pub struct INodeData {
    pub dev: u32,
    pub inum: u32,
    pub ref_count: u32,
    pub valid: bool,

    pub types: u16,
    pub major: u16,
    pub minor: u16,
    pub nlink: u16,

    pub size: u32,
    pub addr: [u32; DIRECTORY_COUNT + 1],
}

impl INodeData {
    const fn new() -> INodeData {
        INodeData {
            dev: 0,
            inum: 0,
            ref_count: 0,
            valid: false,
            types: 0,
            major: 0,
            minor: 0,
            nlink: 0,
            size: 0,
            addr: [0; DIRECTORY_COUNT + 1],
        }
    }
}

pub struct INode {
    lock: SleepLock<()>,
    data: UnsafeCell<INodeData>,
}

impl INode {
    const fn new() -> INode {
        INode {
            lock: SleepLock::new(()),
            data: UnsafeCell::new(INodeData::new()),
        }
    }

    pub fn data(&self) -> &mut INodeData {
        return unsafe { self.data.get().as_mut() }.unwrap();
    }

    // Copy stat information from inode.
    // Caller must hold ip->lock.
    pub fn status(&self) -> FileStatus {
        FileStatus {
            dev: self.data().dev,
            ino: self.data().inum,
            types: self.data().types,
            nlink: self.data().nlink,
            size: self.data().size as u64,
        }
    }

    // Inode content
    //
    // The content (data) associated with each inode is stored
    // in blocks on the disk. The first NDIRECT block numbers
    // are listed in ip->addrs[].  The next NINDIRECT blocks are
    // listed in block ip->addrs[NDIRECT].

    // Return the disk block address of the nth block in inode ip.
    // If there is no such block, bmap allocates one.
    pub fn map(&self, mut bn: u32) -> u32 {
        let log = unsafe { &mut LOG };

        let data = self.data();
        if (bn as usize) < DIRECTORY_COUNT {
            let mut addr = data.addr[bn as usize];
            if addr == 0 {
                addr = Block::alloc(data.dev);
                data.addr[bn as usize] = addr;
            }
            return addr;
        }
        bn -= DIRECTORY_COUNT as u32;

        if (bn as usize) < DIRECTORY_INNER_COUNT {
            // Load indirect block, allocating if necessary.
            let mut addr = data.addr[DIRECTORY_COUNT];
            if addr == 0 {
                addr = Block::alloc(data.dev);
                data.addr[DIRECTORY_COUNT] = addr;
            }

            let bp = BLOCK_CACHE.read(data.dev, addr);
            let a = unsafe { (bp.data() as *mut [u32; 256] as *mut [u32]).as_ref() }.unwrap();
            addr = a[bn as usize];
            if addr == 0 {
                addr = Block::alloc(data.dev);
                data.addr[bn as usize] = addr;
                log.write(&bp);
            }
            BLOCK_CACHE.release(bp);

            return addr;
        }

        panic!("out of range");
    }

    // Read data from inode.
    // Caller must hold ip->lock.
    // If user_dst==1, then dst is a user virtual address;
    // otherwise, dst is a kernel address.
    pub fn read(&self, user_dst: bool, mut dst: usize, mut off: u32, mut n: u32) -> u32 {
        let data = self.data();

        if off > data.size || off + n < off {
            return 0;
        }
        if off + n > data.size {
            n = data.size - off;
        }

        let mut tot = 0;
        while tot < n {
            let bp = BLOCK_CACHE.read(data.dev, self.map(off / BLOCK_SIZE as u32));
            let m = min(n - tot, BLOCK_SIZE as u32 - (off % BLOCK_SIZE as u32));
            if !either_copy_out(user_dst, dst, bp.data() as usize + (off as usize % BLOCK_SIZE), m as usize) {
                BLOCK_CACHE.release(bp);
                tot = 0;
                break;
            }
            BLOCK_CACHE.release(bp);

            tot += m;
            off += m;
            dst += m as usize;
        }
        return tot;
    }


    // Write data to inode.
    // Caller must hold ip->lock.
    // If user_src==1, then src is a user virtual address;
    // otherwise, src is a kernel address.
    // Returns the number of bytes successfully written.
    // If the return value is less than the requested n,
    // there was an error of some kind.
    pub fn write(&self, user_src: bool, mut src: usize, mut off: u32, n: u32) -> u32 {
        let data = self.data();

        let log = unsafe { &mut LOG };

        if off > data.size || off + n < off {
            return 0;
        }
        if off + n > (MAX_FILE_COUNT * BLOCK_SIZE) as u32 {
            return 0;
        }

        let mut tot = 0;
        while tot < n {
            let bp = BLOCK_CACHE.read(data.dev, self.map(off / BLOCK_SIZE as u32));
            let m = min(n - tot, BLOCK_SIZE as u32 - (off % BLOCK_SIZE as u32));
            if !either_copy_in(user_src, bp.data() as usize + (off as usize % BLOCK_SIZE), src, m as usize) {
                BLOCK_CACHE.release(bp);
                break;
            }
            log.write(&bp);
            BLOCK_CACHE.release(bp);

            tot += m;
            off += m;
            src += m as usize;
        }

        if off > data.size {
            data.size = off;
        }

        // write the i-node back to disk even if the size didn't change
        // because the loop above might have called bmap() and added a new
        // block to ip->addrs[].
        self.update();

        return tot;
    }

    // Look for a directory entry in a directory.
    // If found, set *poff to byte offset of entry.
    pub fn dir_lookup(&self, name: &String, poff: *mut u32) -> Option<&INode> {
        let data = self.data();

        assert_eq!(data.types, TYPE_DIR);

        let mut de = Dirent {
            inum: 0,
            name: [0; DIRECTORY_SIZE],
        };

        let size_de = size_of::<Dirent>() as u32;
        for off in (0..data.size).step_by(size_de as usize) {
            if self.read(false, &mut de as *mut _ as usize, off, size_de) != size_de {
                panic!("dirlookup read");
            }
            if de.inum == 0 {
                continue;
            }
            let de_name = unsafe { CStr::from_ptr(&de.name as *const _ as *const c_char) }.to_string_lossy().into_owned();
            if de_name == *name {
                if !poff.is_null() {
                    unsafe {
                        (*poff) = off;
                    }
                }
                return Some(ICACHE.get(data.dev, de.inum as u32));
            }
        }

        return None;
    }

    // Write a new directory entry (name, inum) into the directory dp.
    pub fn dir_link(&self, name: &String, inum: u32) -> bool {
        let data = self.data();

        let ip = self.dir_lookup(name, null_mut());

        // Check that name is not present.
        if ip.is_some() {
            ICACHE.put(self);
            return false;
        }

        let mut de = Dirent {
            inum: 0,
            name: [0; DIRECTORY_SIZE],
        };

        // Look for an empty dirent.
        let size_de = size_of::<Dirent>() as u32;
        let mut off = 0;
        while off < data.size {
            if self.read(false, &de as *const _ as usize, off, size_de) != size_de {
                panic!("dirlink read");
            }
            if de.inum == 0 {
                break;
            }
            off += size_de;
        }

        let c_str = CString::new(name.clone()).expect("CString::new failed");
        let c_bytes = c_str.to_bytes_with_nul();
        assert!(c_bytes.len() <= DIRECTORY_SIZE);

        unsafe {
            ptr::copy(c_bytes as *const _ as *mut [u8; DIRECTORY_SIZE], &mut de.name, 1);
        }
        de.inum = inum as u16;

        if self.write(false, &de as *const _ as usize, off, size_de) != size_de {
            panic!("dirlink");
        }

        true
    }
}

impl INode {
    // Copy a modified in-memory inode to disk.
    // Must be called after every change to an ip->xxx field
    // that lives on disk, since i-node cache is write-through.
    // Caller must hold ip->lock.
    pub fn update(&self) {
        let sb = SUPER_BLOCK.get();
        let log = unsafe { &mut LOG };
        let data = self.data();

        let bp = BLOCK_CACHE.read(data.dev, iblock(data.inum, sb));
        let dip = unsafe { (bp.data() as *mut INodeDisk).offset((data.inum % IPB) as isize).as_mut() }.unwrap();
        dip.types = data.types;
        dip.major = data.major;
        dip.minor = data.minor;
        dip.nlink = data.nlink;
        dip.size = data.size;
        unsafe {
            ptr::copy(&data.addr, &mut dip.addr, 1);
        }
        log.write(&bp);
        BLOCK_CACHE.release(bp);
    }

    // Increment reference count for ip.
    // Returns ip to enable ip = idup(ip1) idiom.
    pub fn dup(&self) -> &INode {
        let guard = ICACHE.nodes.lock();
        let data = self.data();
        data.ref_count += 1;
        drop(guard);
        return self;
    }

    // Lock the given inode.
    // Reads the inode from disk if necessary.
    pub fn lock(&self) -> SleepLockGuard<()> {
        let sb = SUPER_BLOCK.get();
        let data = self.data();

        assert!(data.ref_count >= 1);

        let guard = self.lock.lock();

        if !data.valid {
            let bp = BLOCK_CACHE.read(data.dev, iblock(data.inum, sb));
            let dip = unsafe { (bp.data() as *mut INodeDisk).offset((data.inum % IPB) as isize).as_mut() }.unwrap();

            data.types = dip.types;
            data.major = dip.major;
            data.minor = dip.minor;
            data.nlink = dip.nlink;
            data.size = dip.size;
            unsafe {
                ptr::copy(&dip.addr, &mut data.addr, 1);
            }
            BLOCK_CACHE.release(bp);
            data.valid = true;
            if data.types == 0 {
                panic!("err");
            }
            assert_ne!(data.types, 0);
        }

        guard
    }

    // Unlock the given inode.
    pub fn unlock(&self, guard: SleepLockGuard<()>) {
        assert!(self.data().ref_count >= 1);
        drop(guard);
    }

    pub fn unlock_put(&self, guard: SleepLockGuard<()>) {
        self.unlock(guard);
        ICACHE.put(self);
    }

    // Truncate inode (discard contents).
    // Caller must hold ip->lock.
    pub fn truncate(&self) {
        let data = self.data();
        for i in 0..DIRECTORY_COUNT {
            if data.addr[i] != 0 {
                Block::free(data.dev, data.addr[i]);
                data.addr[i] = 0;
            }
        }

        if data.addr[DIRECTORY_COUNT] != 0 {
            let bp = BLOCK_CACHE.read(data.dev, data.addr[DIRECTORY_COUNT]);
            let a = unsafe { (bp.data() as *mut [u32; 256] as *mut [u32]).as_ref() }.unwrap();
            for i in 0..DIRECTORY_COUNT {
                if a[i] != 0 {
                    Block::free(data.dev, a[i] as u32);
                }
            }
            BLOCK_CACHE.release(bp);
            Block::free(data.dev, data.addr[DIRECTORY_COUNT]);
            data.addr[DIRECTORY_COUNT] = 0;
        }

        data.size = 0;
        self.update();
    }
}

pub static ICACHE: ICache = ICache::new();

pub struct ICache {
    nodes: SpinLock<[INode; MAX_INODE_NUMBER]>,
}

impl ICache {
    const fn new() -> ICache {
        ICache {
            nodes: SpinLock::new(array![_ => INode::new(); MAX_INODE_NUMBER], "icache"),
        }
    }

    // Allocate an inode on device dev.
    // Mark it as allocated by  giving it type type.
    // Returns an unlocked but allocated and referenced inode.
    pub fn alloc(&self, dev: u32, types: u16) -> &INode {
        let sb = SUPER_BLOCK.get();
        let log = unsafe { &mut LOG };
        for inum in 1..sb.inode_number {
            let bp = BLOCK_CACHE.read(dev, iblock(inum, sb));
            let dip = unsafe { (bp.data() as *mut INodeDisk).offset((inum % IPB) as isize).as_mut() }.unwrap();
            if dip.types == 0 { // a free inode
                unsafe {
                    ptr::write_bytes(dip as *mut INodeDisk, 0, 1);
                }
                dip.types = types;
                log.write(&bp);
                BLOCK_CACHE.release(bp);
                return self.get(dev, inum);
            }
            BLOCK_CACHE.release(bp);
        }

        panic!("no inodes");
    }

    pub fn get(&self, dev: u32, inum: u32) -> &INode {
        let mut guard = self.nodes.lock();
        let nodes = &mut *guard;

        let mut empty = None;
        for i in 0..MAX_INODE_NUMBER {
            let ip = &mut nodes[i];
            let data = ip.data();

            if data.ref_count > 0 && data.dev == dev && data.inum == inum {
                data.ref_count += 1;
                return unsafe { (ip as *const INode).as_ref().unwrap() };
            }
            if empty.is_none() && data.ref_count == 0 {
                empty = Some(i);
            }
        }

        if empty.is_none() {
            panic!("no inodes");
        }

        let i = empty.unwrap();
        let ip = &nodes[i];
        let data = ip.data();

        data.dev = dev;
        data.inum = inum;
        data.ref_count = 1;
        data.valid = false;

        return unsafe { (ip as *const INode).as_ref() }.unwrap();
    }

    // Drop a reference to an in-memory inode.
    // If that was the last reference, the inode cache entry can
    // be recycled.
    // If that was the last reference and the inode has no links
    // to it, free the inode (and its content) on disk.
    // All calls to iput() must be inside a transaction in
    // case it has to free the inode.
    pub fn put(&self, inode: &INode) {
        let mut guard = self.nodes.lock();
        let data = inode.data();

        if data.ref_count == 1 && data.valid && data.nlink == 0 {
            // inode has no links and no other references: truncate and free.

            // ip->ref == 1 means no other process can have ip locked,
            // so this acquiresleep() won't block (or deadlock).
            let inode_guard = inode.lock.lock();
            drop(guard);

            inode.truncate();
            data.types = 0;
            inode.update();
            data.valid = false;

            drop(inode_guard);
            guard = self.nodes.lock();
        }

        data.ref_count -= 1;
        drop(guard);
    }
}
