use crate::proc::proc_manager::ProcessControlBlock;
use alloc::collections::VecDeque;
use alloc::sync::Arc;

/// Information for scheduling
pub struct ProcMeta {
    // pub pid: usize,
    /// If time_slice reduce to 0, then the process should give up CPU.
    // pub time_slice: usize,
    pub pcb: Arc<ProcessControlBlock>,
}

// We first implement a naive RR scheduler. haha :).
pub struct Scheduler {
    /// The queue of ready processes.
    queue: VecDeque<ProcMeta>,
}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            queue: VecDeque::new(),
        }
    }
    pub fn push(&mut self, pcb: Arc<ProcessControlBlock>) {
        self.queue.push_back(ProcMeta { pcb });
    }

    pub fn pop(&mut self) -> Option<Arc<ProcessControlBlock>> {
        if let Some(proc_meta) = self.queue.pop_front() {
            Some(proc_meta.pcb)
        } else {
            None
        }
    }
}
