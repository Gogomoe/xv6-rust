use core::convert::TryFrom;
use core::intrinsics::size_of;
use core::ptr;
use core::ptr::null_mut;
use core::sync::atomic::{AtomicBool, fence, Ordering};

use crate::memory::KERNEL_PAGETABLE;
use crate::memory::layout::VIRTIO0;
use crate::memory::PAGE_SIZE;
use crate::process::{CPU_MANAGER, PROCESS_MANAGER};
use crate::spin_lock::SpinLock;

const NUM: usize = 8;
const BLOCK_SIZE: usize = 1024;

#[repr(C)]
struct VRingDesc {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

// chained with another descriptor
const VRING_DESC_F_NEXT: u16 = 1;
// device writes (vs read)
const VRING_DESC_F_WRITE: u16 = 2;

#[repr(C)]
struct VRingUsedElem {
    id: u32,
    len: u32,
}

// read the disk
const VIRTIO_BLK_T_IN: u32 = 0;
// write the disk
const VIRTIO_BLK_T_OUT: u32 = 1;

#[repr(C)]
struct UsedArea {
    flags: u16,
    id: u16,
    elems: [VRingUsedElem; NUM],
}

#[repr(C)]
struct Info {
    buffer: *mut [u8; BLOCK_SIZE],
    status: u8,
    disk: AtomicBool,
}

impl Info {
    const fn new() -> Info {
        Info {
            buffer: null_mut(),
            status: 0,
            disk: AtomicBool::new(false),
        }
    }
}


#[repr(C)]
struct VirtioBlockRequest {
    op_type: u32,
    reserved: u32,
    sector: u64,
}

#[repr(C, align(4096))]
pub struct Disk {
    pages: [u8; 2 * PAGE_SIZE],
    desc: *mut VRingDesc,
    avail: *mut u16,
    used: *mut UsedArea,

    free: [bool; NUM],
    used_idx: u16,

    info: [Info; NUM],
}

pub static mut DISK: Disk = Disk::new();
pub static DISK_LOCK: SpinLock<()> = SpinLock::new((), "disk");

unsafe impl Send for Disk {}

impl Disk {
    const fn new() -> Disk {
        Disk {
            pages: [0; 2 * PAGE_SIZE],
            desc: null_mut(),
            avail: null_mut(),
            used: null_mut(),

            free: [false; NUM],
            used_idx: 0,

            info: array![_ => Info::new(); NUM],
        }
    }

    pub unsafe fn init(&mut self) {
        let mut status = 0;

        if read(VIRTIO_MMIO_MAGIC_VALUE) != 0x74726976 ||
            read(VIRTIO_MMIO_VERSION) != 1 ||
            read(VIRTIO_MMIO_DEVICE_ID) != 2 ||
            read(VIRTIO_MMIO_VENDOR_ID) != 0x554d4551 {
            panic!("could not find virtio disk");
        }

        status |= VIRTIO_CONFIG_S_ACKNOWLEDGE;
        write(VIRTIO_MMIO_STATUS, status);

        status |= VIRTIO_CONFIG_S_DRIVER;
        write(VIRTIO_MMIO_STATUS, status);

        // negotiate features
        let mut features = read(VIRTIO_MMIO_DEVICE_FEATURES);
        features &= !(1 << VIRTIO_BLK_F_RO);
        features &= !(1 << VIRTIO_BLK_F_SCSI);
        features &= !(1 << VIRTIO_BLK_F_CONFIG_WCE);
        features &= !(1 << VIRTIO_BLK_F_MQ);
        features &= !(1 << VIRTIO_F_ANY_LAYOUT);
        features &= !(1 << VIRTIO_RING_F_EVENT_IDX);
        features &= !(1 << VIRTIO_RING_F_INDIRECT_DESC);
        write(VIRTIO_MMIO_DRIVER_FEATURES, features);

        // tell device that feature negotiation is complete.
        status |= VIRTIO_CONFIG_S_FEATURES_OK;
        write(VIRTIO_MMIO_STATUS, status);

        // tell device we're completely ready.
        status |= VIRTIO_CONFIG_S_DRIVER_OK;
        write(VIRTIO_MMIO_STATUS, status);

        write(VIRTIO_MMIO_GUEST_PAGE_SIZE, PAGE_SIZE as u32);

        // initialize queue 0.
        write(VIRTIO_MMIO_QUEUE_SEL, 0);
        let max = read(VIRTIO_MMIO_QUEUE_NUM_MAX);

        if max == 0 {
            panic!("virtio disk has no queue 0");
        }
        if max < NUM as u32 {
            panic!("virtio disk max queue too short");
        }
        write(VIRTIO_MMIO_QUEUE_NUM, NUM as u32);
        ptr::write_bytes(&mut self.pages as *mut [u8; 2 * PAGE_SIZE], 0, self.pages.len());
        write(VIRTIO_MMIO_QUEUE_PFN, u32::try_from(&self.pages as *const _ as usize >> 12).unwrap());

        // desc = pages -- num * VRingDesc
        // avail = pages + 0x80 -- 2 * uint16, then num * uint16
        // used = pages + 4096 -- 2 * uint16, then num * vRingUsedElem

        self.desc = &self.pages as *const _ as *mut VRingDesc;
        self.avail = (self.desc as usize + NUM * size_of::<VRingDesc>()) as *mut u16;
        self.used = (&self.pages as *const _ as usize + PAGE_SIZE) as *mut UsedArea;

        assert_eq!(size_of::<VRingDesc>(), 16);
        assert_eq!(self.desc as usize, &self.pages as *const _ as usize);
        assert_eq!(self.avail as usize, &self.pages as *const _ as usize + 0x80);
        assert_eq!(self.used as usize, &self.pages as *const _ as usize + 4096);

        for i in 0..NUM {
            self.free[i] = true;
        }

        // plic.c and trap.c arrange for interrupts from VIRTIO0_IRQ.
    }

    pub unsafe fn read(&mut self, block_no: usize, data: *mut [u8; BLOCK_SIZE]) {
        self.read_write(block_no, data, false);
    }

    pub unsafe fn write(&mut self, block_no: usize, data: *mut [u8; BLOCK_SIZE]) {
        self.read_write(block_no, data, true);
    }

    unsafe fn read_write(&mut self, block_no: usize, data: *mut [u8; BLOCK_SIZE], is_write: bool) {
        let sector = block_no * BLOCK_SIZE / 512;

        let mut guard = DISK_LOCK.lock();

        let mut idx = self.alloc3_desc();
        while idx.is_none() {
            CPU_MANAGER.my_cpu_mut().sleep(&self.free[0] as *const _ as usize, guard);
            guard = DISK_LOCK.lock();

            idx = self.alloc3_desc();
        }
        let idx = idx.unwrap();

        let buf0 = VirtioBlockRequest {
            op_type: if is_write { VIRTIO_BLK_T_OUT } else { VIRTIO_BLK_T_IN },
            reserved: 0,
            sector: sector as u64,
        };

        // buf0 is on a kernel stack, which is not direct mapped,
        // thus the call to translate().
        let desc0 = self.desc.offset(idx[0] as isize);
        (*desc0).addr = KERNEL_PAGETABLE.lock().translate(&buf0 as *const _ as usize).unwrap() as u64;
        (*desc0).len = size_of::<VirtioBlockRequest>() as u32;
        (*desc0).flags = VRING_DESC_F_NEXT;
        (*desc0).next = idx[1] as u16;

        let desc1 = self.desc.offset(idx[1] as isize);
        (*desc1).addr = KERNEL_PAGETABLE.lock().translate(data as usize).unwrap() as u64;
        (*desc1).len = BLOCK_SIZE as u32;
        (*desc1).flags = VRING_DESC_F_NEXT | if is_write { 0 } else { VRING_DESC_F_WRITE };
        (*desc1).next = idx[2] as u16;

        let info = &mut self.info[idx[0]];
        info.status = 0;

        let desc2 = self.desc.offset(idx[2] as isize);
        (*desc2).addr = &info.status as *const _ as usize as u64;
        (*desc2).len = 1;
        (*desc2).flags = VRING_DESC_F_WRITE;
        (*desc2).next = 0;

        // record struct buf for virtio_disk_intr().
        info.disk.store(true, Ordering::SeqCst);
        info.buffer = data;

        // avail[0] is flags
        // avail[1] tells the device how far to look in avail[2...].
        // avail[2...] are desc[] indices the device should process.
        // we only tell device the first index in our chain of descriptors.

        // self.avail[2 + (self.avail[1] % NUM)] = idx[0];
        *self.avail.offset(2 + (*self.avail.offset(1) as usize % NUM) as isize) = idx[0] as u16;
        fence(Ordering::SeqCst);
        // self.avail[1] = self.avail[1] + 1;
        *self.avail.offset(1) = *self.avail.offset(1) + 1;

        write(VIRTIO_MMIO_QUEUE_NOTIFY, 0);

        // Wait for virtio_disk_intr() to say request has finished.
        while info.disk.load(Ordering::SeqCst) {
            CPU_MANAGER.my_cpu_mut().sleep(data as usize, guard);
            guard = DISK_LOCK.lock();
        }

        info.buffer = null_mut();
        self.free_chain(idx[0]);

        drop(guard);
    }

    unsafe fn alloc3_desc(&mut self) -> Option<[usize; 3]> {
        let mut idx = [0; 3];
        for i in 0..3 {
            match self.alloc_desc() {
                Some(x) => { idx[i] = x }
                None => {
                    for j in 0..i {
                        self.free_desc(idx[j]);
                    }
                    return None;
                }
            }
        }
        Some(idx)
    }

    unsafe fn alloc_desc(&mut self) -> Option<usize> {
        for i in 0..NUM {
            if self.free[i] {
                self.free[i] = false;
                return Some(i);
            }
        }
        None
    }

    unsafe fn free_chain(&mut self, mut i: usize) {
        loop {
            self.free_desc(i);
            if ((*self.desc.offset(i as isize)).flags & VRING_DESC_F_NEXT) != 0 {
                i = (*self.desc.offset(i as isize)).next as usize
            } else {
                break;
            }
        }
    }

    unsafe fn free_desc(&mut self, i: usize) {
        assert!(i < NUM);
        assert!(!self.free[i]);
        (*self.desc.offset(i as isize)).addr = 0;
        self.free[i] = true;
        PROCESS_MANAGER.wakeup(&self.free[0] as *const _ as usize);
    }

    pub unsafe fn intr(&mut self) {
        let guard = DISK_LOCK.lock();

        while (self.used_idx as usize % NUM) != ((*self.used).id as usize % NUM) {
            let id = (*self.used).elems[self.used_idx as usize].id;
            let info = &mut self.info[id as usize];

            assert_eq!(info.status, 0);

            info.disk.store(false, Ordering::SeqCst);
            PROCESS_MANAGER.wakeup(info.buffer as usize);

            self.used_idx = (self.used_idx + 1) % NUM as u16;
        }

        write(VIRTIO_MMIO_INTERRUPT_ACK, read(VIRTIO_MMIO_INTERRUPT_STATUS) & 0x3);

        drop(guard);
    }
}

// 0x74726976
const VIRTIO_MMIO_MAGIC_VALUE: usize = 0x000;
// version; 1 is legacy
const VIRTIO_MMIO_VERSION: usize = 0x004;
// device type; 1 is net, 2 is disk
const VIRTIO_MMIO_DEVICE_ID: usize = 0x008;
// 0x554d4551
const VIRTIO_MMIO_VENDOR_ID: usize = 0x00c;
const VIRTIO_MMIO_DEVICE_FEATURES: usize = 0x010;
const VIRTIO_MMIO_DRIVER_FEATURES: usize = 0x020;
// page size for PFN, write-only
const VIRTIO_MMIO_GUEST_PAGE_SIZE: usize = 0x028;
// select queue, write-only
const VIRTIO_MMIO_QUEUE_SEL: usize = 0x030;
// max size of current queue, read-only
const VIRTIO_MMIO_QUEUE_NUM_MAX: usize = 0x034;
// size of current queue, write-only
const VIRTIO_MMIO_QUEUE_NUM: usize = 0x038;
// used ring alignment, write-only
const VIRTIO_MMIO_QUEUE_ALIGN: usize = 0x03c;
// physical page number for queue, read/write
const VIRTIO_MMIO_QUEUE_PFN: usize = 0x040;
// ready bit
const VIRTIO_MMIO_QUEUE_READY: usize = 0x044;
// write-only
const VIRTIO_MMIO_QUEUE_NOTIFY: usize = 0x050;
// read-only
const VIRTIO_MMIO_INTERRUPT_STATUS: usize = 0x060;
// write-only
const VIRTIO_MMIO_INTERRUPT_ACK: usize = 0x064;
// read/write
const VIRTIO_MMIO_STATUS: usize = 0x070;

// status register bits, from qemu virtio_config.h
const VIRTIO_CONFIG_S_ACKNOWLEDGE: u32 = 1;
const VIRTIO_CONFIG_S_DRIVER: u32 = 2;
const VIRTIO_CONFIG_S_DRIVER_OK: u32 = 4;
const VIRTIO_CONFIG_S_FEATURES_OK: u32 = 8;

const VIRTIO_BLK_F_RO: u8 = 5;    /* Disk is read-only */
const VIRTIO_BLK_F_SCSI: u8 = 7;    /* Supports scsi command passthru */
const VIRTIO_BLK_F_CONFIG_WCE: u8 = 11;    /* Writeback mode available in config */
const VIRTIO_BLK_F_MQ: u8 = 12;    /* support more than one vq */
const VIRTIO_F_ANY_LAYOUT: u8 = 27;
const VIRTIO_RING_F_INDIRECT_DESC: u8 = 28;
const VIRTIO_RING_F_EVENT_IDX: u8 = 29;

#[inline]
unsafe fn read(offset: usize) -> u32 {
    let src = (Into::<usize>::into(VIRTIO0) + offset) as *const u32;
    ptr::read_volatile(src)
}

#[inline]
unsafe fn write(offset: usize, data: u32) {
    let dst = (Into::<usize>::into(VIRTIO0) + offset) as *mut u32;
    ptr::write_volatile(dst, data);
}
