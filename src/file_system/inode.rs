use core::intrinsics::size_of;
use core::ptr;

use spin::Mutex;

use crate::file_system::{Block, BLOCK_CACHE, BLOCK_SIZE, DIRECTORY_COUNT, LOG, SUPER_BLOCK, SuperBlock};
use crate::param::MAX_INODE_NUMBER;
use crate::sleep_lock::{SleepLock, SleepLockGuard};

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
    nodes: Mutex<[INode; MAX_INODE_NUMBER]>,
}

impl ICache {
    const fn new() -> ICache {
        ICache {
            nodes: Mutex::new(array![_ => INode::new(); MAX_INODE_NUMBER]),
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
