use core::cmp::min;
use core::intrinsics::size_of;
use core::ptr;
use core::ptr::null_mut;

use crate::file_system::{Block, BLOCK_CACHE, BLOCK_SIZE, DIRECTORY_COUNT, DIRECTORY_SIZE, Dirent, LOG, MAX_FILE_COUNT, SUPER_BLOCK, SuperBlock};
use crate::file_system::directory::{FileStatus, TYPE_DIR};
use crate::memory::{either_copy_in, either_copy_out};
use crate::param::MAX_INODE_NUMBER;
use crate::sleep_lock::{SleepLock, SleepLockGuard};
use crate::spin_lock::SpinLock;

// Inodes per block
const IPB: usize = BLOCK_SIZE / size_of::<INodeDisk>();

#[inline]
fn iblock(i: usize, sb: &SuperBlock) -> usize {
    i / IPB + sb.inode_start
}

pub struct INode {
    lock: SleepLock<()>,
    dev: usize,
    inum: usize,
    ref_count: usize,
    valid: bool,

    types: u16,
    major: u16,
    minor: u16,
    nlink: u16,

    size: usize,
    addr: [usize; DIRECTORY_COUNT + 1],

}

impl INode {
    const fn new() -> INode {
        INode {
            lock: SleepLock::new(()),
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

    // Copy stat information from inode.
    // Caller must hold ip->lock.
    pub fn status(&self) -> FileStatus {
        FileStatus {
            dev: self.dev,
            ino: self.inum,
            types: self.types,
            nlink: self.nlink,
            size: self.size,
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
    pub fn map(&mut self, mut bn: usize) -> usize {
        let log = unsafe { &mut LOG };

        if bn < DIRECTORY_COUNT {
            let mut addr = self.addr[bn];
            if addr == 0 {
                addr = Block::alloc(self.dev);
                self.addr[bn] = addr;
            }
            return addr;
        }
        bn -= DIRECTORY_COUNT;

        if bn < DIRECTORY_COUNT {
            // Load indirect block, allocating if necessary.
            let mut addr = self.addr[DIRECTORY_COUNT];
            if addr == 0 {
                addr = Block::alloc(self.dev);
                self.addr[DIRECTORY_COUNT] = addr;
            }

            let bp = BLOCK_CACHE.read(self.dev, addr);
            let a = unsafe { (bp.data() as *mut [usize; 128] as *mut [usize]).as_ref() }.unwrap();
            addr = a[bn];
            if addr == 0 {
                addr = Block::alloc(self.dev);
                self.addr[bn] = addr;
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
    pub fn read(&mut self, user_dst: bool, mut dst: usize, mut off: usize, mut n: usize) -> usize {
        if off > self.size || off + n < off {
            return 0;
        }
        if off + n > self.size {
            n = self.size - off;
        }

        let mut tot = 0;
        while tot < n {
            let bp = BLOCK_CACHE.read(self.dev, self.map(off / BLOCK_SIZE));
            let m = min(n - tot, BLOCK_SIZE - off % BLOCK_SIZE);
            if !either_copy_out(user_dst, dst, bp.data() as usize + (off % BLOCK_SIZE), m) {
                BLOCK_CACHE.release(bp);
                tot = 0;
                break;
            }
            BLOCK_CACHE.release(bp);

            tot += m;
            off += m;
            dst += m;
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
    pub fn write(&mut self, user_src: bool, mut src: usize, mut off: usize, n: usize) -> usize {
        let log = unsafe { &mut LOG };

        if off > self.size || off + n < off {
            return 0;
        }
        if off + n > MAX_FILE_COUNT * BLOCK_SIZE {
            return 0;
        }

        let mut tot = 0;
        while tot < n {
            let bp = BLOCK_CACHE.read(self.dev, self.map(off / BLOCK_SIZE));
            let m = min(n - tot, BLOCK_SIZE - off % BLOCK_SIZE);
            if !either_copy_in(user_src, bp.data() as usize + (off % BLOCK_SIZE), src, m) {
                BLOCK_CACHE.release(bp);
                break;
            }
            log.write(&bp);
            BLOCK_CACHE.release(bp);

            tot += m;
            off += m;
            src += m;
        }

        if off > self.size {
            self.size = off;
        }

        // write the i-node back to disk even if the size didn't change
        // because the loop above might have called bmap() and added a new
        // block to ip->addrs[].
        ICache::update(self);

        return tot;
    }

    // Look for a directory entry in a directory.
    // If found, set *poff to byte offset of entry.
    pub fn dir_lookup(&mut self, name: &[u8], poff: *mut usize) -> Option<&INode> {
        assert_eq!(self.types, TYPE_DIR);

        let mut de = Dirent {
            inum: 0,
            name: [0; DIRECTORY_SIZE],
        };

        let size_de = size_of::<Dirent>();
        for off in (0..self.size).step_by(size_de) {
            if self.read(false, &mut de as *mut _ as usize, off, size_de) != size_de {
                panic!("dirlookup read");
            }
            if de.inum == 0 {
                continue;
            }
            if de.name == *name {
                if !poff.is_null() {
                    unsafe {
                        (*poff) = off;
                    }
                }
                return Some(ICACHE.get(self.dev, de.inum as usize));
            }
        }

        return None;
    }

    pub fn dir_link(&mut self, name: &[u8], inum: usize) -> Option<()> {
        let ip = self.dir_lookup(name, null_mut());

        // Check that name is not present.
        if ip.is_some() {
            ICACHE.put(self);
            return None;
        }

        let mut de = Dirent {
            inum: 0,
            name: [0; DIRECTORY_SIZE],
        };

        // Look for an empty dirent.
        let size_de = size_of::<Dirent>();
        let mut off = 0;
        while off < self.size {
            if self.read(false, &de as *const _ as usize, off, size_de) != size_de {
                panic!("dirlink read");
            }
            if de.inum == 0 {
                break;
            }
            off += size_de;
        }

        unsafe {
            ptr::copy(name as *const _ as *mut [u8; DIRECTORY_SIZE], &mut de.name, 1);
        }
        de.inum = inum as u16;

        if self.write(false, &de as *const _ as usize, off, size_de) != size_de {
            panic!("dirlink");
        }

        Some(())
    }
}

#[repr(C)]
pub struct INodeDisk {
    types: u16,
    major: u16,
    minor: u16,
    nlink: u16,

    size: usize,
    addr: [usize; DIRECTORY_COUNT + 1],
}

impl INodeDisk {
    const fn new() -> INodeDisk {
        INodeDisk {
            types: 0,
            major: 0,
            minor: 0,
            nlink: 0,
            size: 0,
            addr: [0; DIRECTORY_COUNT + 1],
        }
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
    pub fn alloc(&self, dev: usize, types: u16) -> &INode {
        let sb = unsafe { &SUPER_BLOCK };
        let log = unsafe { &mut LOG };
        for inum in 1..sb.inode_number {
            let bp = BLOCK_CACHE.read(dev, iblock(inum, sb));
            let dip = unsafe { (bp.data() as *mut INodeDisk).offset((inum % IPB) as isize).as_mut() }.unwrap();
            if dip.types == 0 { // a free inode
                unsafe {
                    ptr::write_bytes(dip as *mut INodeDisk, 0, size_of::<INodeDisk>());
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

    fn get(&self, dev: usize, inum: usize) -> &INode {
        let mut guard = self.nodes.lock();
        let nodes = &mut *guard;

        let mut empty = None;
        for i in 0..MAX_INODE_NUMBER {
            let ip = &mut nodes[i];

            if ip.ref_count > 0 && ip.dev == dev && ip.inum == inum {
                ip.ref_count += 1;
                return unsafe { (ip as *const INode).as_ref().unwrap() };
            }
            if empty.is_none() && ip.ref_count == 0 {
                empty = Some(i);
            }
        }

        if empty.is_none() {
            panic!("no inodes");
        }

        let i = empty.unwrap();
        let ip = &mut nodes[i];

        ip.dev = dev;
        ip.inum = inum;
        ip.ref_count = 1;
        ip.valid = false;

        return unsafe { (ip as *const INode).as_ref().unwrap() };
    }

    // Copy a modified in-memory inode to disk.
    // Must be called after every change to an ip->xxx field
    // that lives on disk, since i-node cache is write-through.
    // Caller must hold ip->lock.
    pub fn update(inode: &INode) {
        let sb = unsafe { &SUPER_BLOCK };
        let log = unsafe { &mut LOG };

        let bp = BLOCK_CACHE.read(inode.dev, iblock(inode.inum, sb));
        let dip = unsafe { (bp.data() as *mut INodeDisk).offset((inode.inum % IPB) as isize).as_mut() }.unwrap();
        dip.types = inode.types;
        dip.major = inode.major;
        dip.minor = inode.minor;
        dip.nlink = inode.nlink;
        dip.size = inode.size;
        unsafe {
            ptr::copy(&inode.addr, &mut dip.addr, 1);
        }
        log.write(&bp);
        BLOCK_CACHE.release(bp);
    }

    // Increment reference count for ip.
    // Returns ip to enable ip = idup(ip1) idiom.
    pub fn dup(inode: &mut INode) -> &mut INode {
        let guard = inode.lock.lock();
        (*inode).ref_count += 1;
        drop(guard);
        return inode;
    }

    // Lock the given inode.
    // Reads the inode from disk if necessary.
    pub fn lock(inode: &mut INode) -> SleepLockGuard<()> {
        let sb = unsafe { &SUPER_BLOCK };

        assert!(inode.ref_count >= 1);

        let guard = inode.lock.lock();

        if !inode.valid {
            let bp = BLOCK_CACHE.read(inode.dev, iblock(inode.inum, sb));
            let dip = unsafe { (bp.data() as *mut INodeDisk).offset((inode.inum % IPB) as isize).as_mut() }.unwrap();

            inode.types = dip.types;
            inode.major = dip.major;
            inode.minor = dip.minor;
            inode.nlink = dip.nlink;
            inode.size = dip.size;
            unsafe {
                ptr::copy(&dip.addr, &mut inode.addr, 1);
            }
            BLOCK_CACHE.release(bp);
            inode.valid = true;
            assert_ne!(inode.types, 0);
        }

        guard
    }

    // Unlock the given inode.
    pub fn unlock(inode: &INode, guard: SleepLockGuard<()>) {
        assert!(inode.ref_count >= 1);
        drop(guard);
    }

    // Drop a reference to an in-memory inode.
    // If that was the last reference, the inode cache entry can
    // be recycled.
    // If that was the last reference and the inode has no links
    // to it, free the inode (and its content) on disk.
    // All calls to iput() must be inside a transaction in
    // case it has to free the inode.
    pub fn put(&self, inode: &mut INode) {
        let mut guard = self.nodes.lock();

        if inode.ref_count == 1 && inode.valid && inode.nlink == 0 {
            // inode has no links and no other references: truncate and free.

            // ip->ref == 1 means no other process can have ip locked,
            // so this acquiresleep() won't block (or deadlock).
            let inode_guard = inode.lock.lock();
            drop(guard);

            ICache::truncate(unsafe { (inode as *const _ as *mut INode).as_mut() }.unwrap());
            inode.types = 0;
            ICache::update(inode);
            inode.valid = false;

            drop(inode_guard);
            guard = self.nodes.lock();
        }

        inode.ref_count -= 1;
        drop(guard);
    }

    // Truncate inode (discard contents).
    // Caller must hold ip->lock.
    pub fn truncate(inode: &mut INode) {
        for i in 0..DIRECTORY_COUNT {
            if inode.addr[i] != 0 {
                Block::free(inode.dev, inode.addr[i]);
                inode.addr[i] = 0;
            }
        }

        if inode.addr[DIRECTORY_COUNT] != 0 {
            let bp = BLOCK_CACHE.read(inode.dev, inode.addr[DIRECTORY_COUNT]);
            let a = unsafe { (bp.data() as *mut [usize; 128] as *mut [usize]).as_ref() }.unwrap();
            for i in 0..DIRECTORY_COUNT {
                if a[i] != 0 {
                    Block::free(inode.dev, a[i]);
                }
            }
            BLOCK_CACHE.release(bp);
            Block::free(inode.dev, inode.addr[DIRECTORY_COUNT]);
            inode.addr[DIRECTORY_COUNT] = 0;
        }

        inode.size = 0;
        ICache::update(inode);
    }
}
