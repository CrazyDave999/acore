
use super::pcb::ProcessControlBlock;

use crate::proc::scheduler::Scheduler;

use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use crate::proc::proc_ctx::ProcContext;
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;

pub struct ProcessManager {
    cur: Option<Arc<ProcessControlBlock>>,
    procs: Vec<Arc<ProcessControlBlock>>,
    scheduler: Scheduler,
}

impl ProcessManager {

    pub fn new() -> Self {
        ProcessManager {
            cur: None,
            procs: Vec::new(),
            scheduler: Scheduler::new(),
        }
    }
}

lazy_static!{
    pub static ref PROC_MANAGER: UPSafeCell<ProcessManager> = unsafe { UPSafeCell::new(ProcessManager::new()) };
}

/// Get current process's root_ppn of the page table
pub fn cur_user_token() -> usize {
    PROC_MANAGER.exclusive_access().cur.as_ref().unwrap().token()
}

/// Get current running process's pcb
pub fn cur_proc() -> Option<Arc<ProcessControlBlock>> {
    PROC_MANAGER.exclusive_access().cur.as_ref().map(Arc::clone)
}

/// Get mutable reference to current process's trap context
pub fn cur_trap_ctx() -> &'static mut TrapContext {
    cur_proc().unwrap().trap_ctx_ppn.get_mut()
}

/// Suspend current process and switch to a ready one
pub fn switch_proc() {
    let mut scheduler = &PROC_MANAGER.exclusive_access().scheduler;
    scheduler.
}
