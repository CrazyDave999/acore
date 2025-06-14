use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use crate::proc::{block_thread, get_cur_thread, switch_thread, wakeup_thread, ThreadControlBlock};
use crate::sync::UPSafeCell;

pub trait Mutex: Sync + Send {
    fn lock(&self);
    fn unlock(&self);
}

pub struct SpinMutex {
    is_locked: UPSafeCell<bool>,
}
impl SpinMutex {
    pub fn new() -> Self {
        Self {
            is_locked: unsafe { UPSafeCell::new(false) },
        }
    }
}

impl Mutex for SpinMutex {
    fn lock(&self) {
        loop {
            let mut is_locked = self.is_locked.exclusive_access();
            if *is_locked {
                drop(is_locked);
                // yield to allow other tasks to run
                switch_thread();
                continue;
            } else {
                *is_locked = true;
                return;
            }
        }
    }

    fn unlock(&self) {
        let mut is_locked = self.is_locked.exclusive_access();
        *is_locked = false;
    }
}

pub struct BlockedMutex {
    inner: UPSafeCell<BlockedMutexInner>,
}

pub struct BlockedMutexInner {
    is_locked: bool,
    wait_queue: VecDeque<Arc<ThreadControlBlock>>,
}

impl BlockedMutex {
    pub fn new() -> Self {
        Self {
            inner: unsafe {
                UPSafeCell::new(BlockedMutexInner {
                    is_locked: false,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }
}
impl Mutex for BlockedMutex {
    fn lock(&self) {
        let mut inner = self.inner.exclusive_access();
        if inner.is_locked {
            inner.wait_queue.push_back(get_cur_thread().unwrap());
            drop(inner);
            block_thread();
        } else {
            inner.is_locked = true;
        }
    }
    fn unlock(&self) {
        let mut inner = self.inner.exclusive_access();
        assert!(inner.is_locked, "Mutex is not locked");
        if let Some(tcb) = inner.wait_queue.pop_front(){
            wakeup_thread(tcb);
        } else {
            inner.is_locked = false;
        }
    }
}