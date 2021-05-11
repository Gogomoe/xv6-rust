use core::cell::{Cell, UnsafeCell};
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, fence, Ordering};

use crate::process::{cpu_id, CPU_MANAGER};

pub struct SpinLock<T: ?Sized> {
    lock: AtomicBool,
    name: &'static str,
    cpuid: Cell<isize>,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub const fn new(data: T, name: &'static str) -> SpinLock<T> {
        SpinLock {
            lock: AtomicBool::new(false),
            name,
            cpuid: Cell::new(-1),
            data: UnsafeCell::new(data),
        }
    }
}

impl<T: ?Sized> SpinLock<T> {
    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        self.acquire();
        SpinLockGuard {
            lock: &self,
            data: unsafe { &mut *self.data.get() },
        }
    }

    unsafe fn holding(&self) -> bool {
        self.lock.load(Ordering::Relaxed) && (self.cpuid.get() == cpu_id() as isize)
    }

    fn acquire(&self) {
        CPU_MANAGER.my_cpu_mut().push_off();
        if unsafe { self.holding() } {
            panic!("spinlock {} acquire", self.name);
        }
        while self.lock.compare_and_swap(false, true, Ordering::Acquire) {}
        fence(Ordering::SeqCst);
        self.cpuid.set(cpu_id() as isize);
    }

    fn release(&self) {
        if unsafe { !self.holding() } {
            panic!("spinlock {} release", self.name);
        }
        self.cpuid.set(-1);
        fence(Ordering::SeqCst);
        self.lock.store(false, Ordering::Release);
        CPU_MANAGER.my_cpu_mut().pop_off();
    }

    /// A hole for fork_ret() to release a proc's excl lock
    pub unsafe fn unlock(&self) {
        self.release();
    }
}

pub struct SpinLockGuard<'a, T: ?Sized> {
    lock: &'a SpinLock<T>,
    data: &'a mut T,
}

impl<'a, T: ?Sized> Deref for SpinLockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &*self.data
    }
}

impl<'a, T: ?Sized> DerefMut for SpinLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.data
    }
}

impl<'a, T: ?Sized> Drop for SpinLockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.release();
    }
}

impl<'a, T> SpinLockGuard<'a, T> {
    pub unsafe fn holding(&self) -> bool {
        self.lock.holding()
    }
}