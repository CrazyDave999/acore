use crate::proc::{block_thread, get_cur_thread, wakeup_thread, ThreadControlBlock};
use crate::sync::{Mutex, UPSafeCell};
use alloc::collections::VecDeque;
use alloc::sync::Arc;

pub struct Condvar {
    pub inner: UPSafeCell<CondvarInner>,
}
pub struct CondvarInner {
    pub wait_queue: VecDeque<Arc<ThreadControlBlock>>,
}

impl Condvar{
    pub fn new() -> Self {
        Self {
            inner: unsafe {
                UPSafeCell::new(CondvarInner {
                    wait_queue: VecDeque::new(),
                })
            }
        }
    }
    pub fn signal(&self) {
        let mut inner = self.inner.exclusive_access();
        if let Some(thread) = inner.wait_queue.pop_front() {
            wakeup_thread(thread);
        }
    }
    pub fn wait(&self, mutex: Arc<dyn Mutex>) {
        // println!("Condvar wait, mutex will be unlocked");
        mutex.unlock();
        // println!("Condvar wait, mutex is unlocked now");
        let mut inner = self.inner.exclusive_access();
        inner.wait_queue.push_back(get_cur_thread().unwrap());
        drop(inner);
        // println!("Condvar wait, mutex will be unlocked");
        block_thread();
        // println!("Condvar wait done, mutex will be locked again");
        mutex.lock();
        // println!("got mutex after condvar wait");
    }
}