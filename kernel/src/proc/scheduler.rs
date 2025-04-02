use alloc::collections::VecDeque;
use alloc::sync::Arc;
use crate::proc::proc_manager::ProcessControlBlock;

/// Information for scheduling
pub struct ProcMeta {
    pid: usize,
    /// If time_slice reduce to 0, then the process should give up CPU.
    time_slice: usize,
    pcb: Arc<ProcessControlBlock>,
}


// We first implement a naive scheduler. haha :).
pub struct Scheduler {
    /// The queue of ready processes.
    queue: VecDeque<ProcMeta>,
}

impl Scheduler {
    pub fn push(&mut self, pcb: Arc<ProcessControlBlock>) {
        todo!()
    }

    pub fn pop(&mut self) -> Option<Arc<ProcessControlBlock>> {
        todo!()
    }
}