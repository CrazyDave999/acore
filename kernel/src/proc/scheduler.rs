use crate::proc::thread::ThreadControlBlock;
use alloc::collections::VecDeque;
use alloc::sync::Arc;

/// Information for scheduling
pub struct ThreadMeta {
    // pub pid: usize,
    /// If time_slice reduce to 0, then the process should give up CPU.
    // pub time_slice: usize,
    pub tcb: Arc<ThreadControlBlock>,
}

// We first implement a naive RR scheduler. haha :).
pub struct Scheduler {
    /// The queue of ready processes.
    queue: VecDeque<ThreadMeta>,
}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            queue: VecDeque::new(),
        }
    }
    pub fn push(&mut self, tcb: Arc<ThreadControlBlock>) {
        self.queue.push_back(ThreadMeta { tcb });
    }

    pub fn pop(&mut self) -> Option<Arc<ThreadControlBlock>> {
        if let Some(thr_meta) = self.queue.pop_front() {
            Some(thr_meta.tcb)
        } else {
            None
        }
    }
    pub fn remove(&mut self, tcb: Arc<ThreadControlBlock>) {
        self.queue.retain(|meta| {
            Arc::as_ptr(&meta.tcb) != Arc::as_ptr(&tcb)
        });
    }
}
