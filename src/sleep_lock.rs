use core::cell::{Cell, UnsafeCell};
use core::ops::{Deref, DerefMut};

use crate::process::{CPU_MANAGER, PROCESS_MANAGER};
use crate::spin_lock::SpinLock;

pub struct SleepLock<T: ?Sized> {
    lock: SpinLock<()>,
    locked: Cell<bool>,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Sync for SleepLock<T> {}

impl<T> SleepLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            lock: SpinLock::new((), "sleeplock"),
            locked: Cell::new(false),
            data: UnsafeCell::new(data),
        }
    }
}

impl<T: ?Sized> SleepLock<T> {
    pub fn lock(&self) -> SleepLockGuard<'_, T> {
        let mut guard = self.lock.lock();
        while self.locked.get() {
            CPU_MANAGER.my_cpu_mut().sleep(self.locked.as_ptr() as usize, guard);
            guard = self.lock.lock();
        }
        self.locked.set(true);
        drop(guard);
        SleepLockGuard {
            lock: &self,
            data: unsafe { &mut *self.data.get() },
        }
    }

    fn unlock(&self) {
        let guard = self.lock.lock();
        self.locked.set(false);
        self.wakeup();
        drop(guard)
    }

    fn wakeup(&self) {
        PROCESS_MANAGER.wakeup(self.locked.as_ptr() as usize);
    }
}

pub struct SleepLockGuard<'a, T: ?Sized> {
    lock: &'a SleepLock<T>,
    data: &'a mut T,
}

impl<'a, T: ?Sized> Deref for SleepLockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &*self.data
    }
}

impl<'a, T: ?Sized> DerefMut for SleepLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.data
    }
}

impl<'a, T: ?Sized> Drop for SleepLockGuard<'a, T> {
    /// The dropping of the SpinLockGuard will call spinlock's release_lock(),
    /// through its reference to its original spinlock.
    fn drop(&mut self) {
        self.lock.unlock();
    }
}
