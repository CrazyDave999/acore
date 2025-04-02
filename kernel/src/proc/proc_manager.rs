use alloc::rc::Weak;
use alloc::sync::Arc;
use alloc::vec::Vec;
use crate::proc::pid::PIDGuard;
use crate::proc::scheduler::Scheduler;
use super::proc_ctx::ProcContext;
use crate::mm::PhysPageNum;
use crate::mm::MemoryManager;
use super::__switch;

pub enum ProcessState {
    Ready,
    Running,
    Blocked,
    // Zombie,
}
pub struct ProcessControlBlock {
    pub pid: PIDGuard,
    pub state: ProcessState,
    pub trap_cx_ppn: PhysPageNum,
    pub proc_ctx: ProcContext,
    pub parent: Option<Weak<ProcessControlBlock>>,
    pub children: Vec<Arc<ProcessControlBlock>>,
    pub exit_code: i32,
    pub mm: MemoryManager,
}

impl ProcessControlBlock {
    pub fn from_elf(data: &mut[u8]) -> Self {
        todo!()
    }
}

pub struct ProcessManager {
    cur: Option<Arc<ProcessControlBlock>>,
    procs: Vec<Arc<ProcessControlBlock>>,
    scheduler: Scheduler,
}

impl ProcessManager {
    /// insert a new process
    pub fn push(&mut self, pcb: Arc<ProcessControlBlock>) {
        self.procs.push(pcb);
    }
    /// will only be called by the trap handler
    pub fn switch_proc(&mut self) {
        // get the context of current proc and the next proc, then call __switch
        let cur_proc = Arc::clone(self.cur.as_ref().unwrap());
        let next_proc = self.scheduler.pop().unwrap();
        let cur_proc_ctx = &cur_proc.proc_ctx;
        let next_proc_ctx = &next_proc.proc_ctx;
        self.cur = Some(next_proc);
        self.scheduler.push(cur_proc);
        unsafe {
            __switch(cur_proc_ctx as *const ProcContext as *mut ProcContext, next_proc_ctx as *const ProcContext);
        }
    }
}