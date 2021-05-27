use alloc::alloc::{GlobalAlloc, Layout};
use core::mem::size_of;
use crate::*;

#[repr(C)]
struct Header {
    ptr: Option<*mut Header>,
    size: usize,
}

#[global_allocator]
#[allow(non_upper_case_globals)]
static mut base: Header = Header { ptr: None, size: 0 };
#[allow(non_upper_case_globals)]
static mut freep: Option<*mut Header> = None;

unsafe impl GlobalAlloc for Header {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        malloc(layout.size())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        free(ptr);
    }
}

pub unsafe fn free(ap: *mut u8) {
    let bp = (ap as *mut Header).sub(1);
    let mut p = freep.unwrap();
    while !(bp > p && bp < (*p).ptr.unwrap()) {
        if p >= (*p).ptr.unwrap() && (bp > p || bp < (*p).ptr.unwrap()) {
            break;
        }
        p = (*p).ptr.unwrap();
    }

    if bp.add((*bp).size) == (*p).ptr.unwrap() {
        (*bp).size += (*(*p).ptr.unwrap()).size;
        (*bp).ptr = (*(*p).ptr.unwrap()).ptr;
    } else {
        (*bp).ptr = (*p).ptr;
    }

    if p.add((*p).size) == bp {
        (*p).size += (*bp).size;
        (*p).ptr = (*bp).ptr;
    } else {
        (*p).ptr = Some(bp);
    }

    freep = Some(p);
}

unsafe fn morecore(mut nu: usize) -> Option<*mut Header> {
    if nu < 4096 {
        nu = 4096;
    }
    let p: *mut u8 = sbrk(nu * size_of::<Header>());
    if p as isize == -1 {
        return None;
    }
    let hp = p as *mut _ as *mut Header;
    (*hp).size = nu;
    free(hp.add(1) as *mut u8);
    freep
}

pub unsafe fn malloc(nbytes: usize) -> *mut u8 {
    let nunits = (nbytes + size_of::<Header>() - 1) / size_of::<Header>() + 1;
    
    if let None = freep {
        base.size = 0;
        base.ptr = Some(&mut base as *mut Header);
        freep = Some(&mut base as *mut Header);
    }

    let mut prevp: *mut Header = &mut base as *mut Header;
    let mut p: *mut Header = (*prevp).ptr.unwrap();

    loop {
        if (*p).size >= nunits {
            if (*p).size == nunits {
                (*prevp).ptr = (*p).ptr;
            } else {
                (*p).size -= nunits;
                p = p.add((*p).size);
                (*p).size = nunits;
            }
            freep = Some(prevp);
            return p.add(1) as *mut u8;
        }
        if p == freep.unwrap() {
            if let Some(x) = morecore(nunits) {
                p = x;
            } else {
                return 0 as *mut u8;
            }
        }
    }
}

#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("alloc error: {:?}", layout)
}
