use linked_list_allocator::LockedHeap;

use crate::memory::layout::{KERNEL_HEAP_SIZE, KERNEL_HEAP_START};

#[global_allocator]
static KERNEL_HEAP: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("alloc error: {:?}", layout)
}

pub fn kernel_heap_init() {
    unsafe {
        KERNEL_HEAP.lock().init(KERNEL_HEAP_START, KERNEL_HEAP_SIZE);
    }
}
