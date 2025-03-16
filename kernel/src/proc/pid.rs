use alloc::vec::Vec;
use lazy_static::lazy_static;
use crate::sync::UPSafeCell;

pub struct PIDAllocator {
    cap: usize,
    recycled: Vec<usize>,
}

pub struct PIDGuard(usize);

impl PIDAllocator {
    pub fn new() -> Self {
        PIDAllocator {
            cap: 0,
            recycled: Vec::new(),
        }
    }
    pub fn alloc(&mut self) -> PIDGuard {
        if let Some(pid) = self.recycled.pop() {
            PIDGuard(pid)
        } else {
            let pid = self.cap;
            self.cap += 1;
            PIDGuard(pid)
        }
    }
    pub fn dealloc(&mut self, pid: usize) {
        assert!(pid < self.cap);
        assert!(self.recycled.iter().all(|&p| p != pid), "pid {} has been recycled", pid);
        self.recycled.push(pid);
    }
}

lazy_static! {
    pub static ref PID_ALLOCATOR: UPSafeCell<PIDAllocator> = unsafe { UPSafeCell::new(PIDAllocator::new()) };
}

impl Drop for PIDGuard{
    fn drop(&mut self) {
        PID_ALLOCATOR.exclusive_access().dealloc(self.0);
    }
}

pub fn pid_alloc() -> PIDGuard {
    PID_ALLOCATOR.exclusive_access().alloc()
}