use super::pcb::{ProcessControlBlock, ProcessState};

use crate::proc::scheduler::Scheduler;

use crate::proc::proc_ctx::ProcContext;
use crate::proc::switch::__switch;
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use alloc::sync::Arc;
use lazy_static::lazy_static;

pub struct ProcessManager {
    cur: Option<Arc<ProcessControlBlock>>,
    // procs: Vec<Arc<ProcessControlBlock>>,
    scheduler: Scheduler,
}

impl ProcessManager {
    pub fn new() -> Self {
        ProcessManager {
            cur: None,
            // procs: Vec::new(),
            scheduler: Scheduler::new(),
        }
    }
}

lazy_static! {
    pub static ref PROC_MANAGER: UPSafeCell<ProcessManager> =
        unsafe { UPSafeCell::new(ProcessManager::new()) };
}

/// Get current process's root_ppn of the page table
pub fn get_cur_user_token() -> usize {
    PROC_MANAGER
        .exclusive_access()
        .cur
        .as_ref()
        .unwrap()
        .token()
}

/// Get current running process's pcb
pub fn get_cur_proc() -> Option<Arc<ProcessControlBlock>> {
    PROC_MANAGER.exclusive_access().cur.as_ref().map(Arc::clone)
}

/// Get mutable reference to current process's trap context
pub fn get_cur_trap_ctx() -> &'static mut TrapContext {
    get_cur_proc()
        .unwrap()
        .exclusive_access()
        .trap_ctx_ppn
        .get_mut()
}

/// Suspend current process and switch to a ready one
pub fn switch_proc() -> ! {
    let scheduler = &mut PROC_MANAGER.exclusive_access().scheduler;
    if let Some(next_proc) = scheduler.pop() {
        let cur_proc = get_cur_proc().unwrap();
        next_proc.exclusive_access().state = ProcessState::Running;
        cur_proc.exclusive_access().state = ProcessState::Ready;
        let next_trap_ctx: *const ProcContext = next_proc.exclusive_access().trap_ctx_ppn.get_mut();
        let cur_trap_ctx: *mut ProcContext = cur_proc.exclusive_access().trap_ctx_ppn.get_mut();
        scheduler.push(cur_proc);
        unsafe {
            __switch(cur_trap_ctx, next_trap_ctx)
        }
    }
    panic!("No ready process to switch to!");
}

/// Push a newly created process to the scheduler's ready queue.
pub fn push_proc(proc: Arc<ProcessControlBlock>) {
    &PROC_MANAGER.exclusive_access().scheduler.push(proc);
}

pub fn launch(proc: Arc<ProcessControlBlock>) -> ! {
    let mut inner = PROC_MANAGER.exclusive_access();
    inner.cur = Some(proc);
    inner.cur.as_ref().unwrap().set_state(ProcessState::Running);
    drop(inner);
    let cur_proc = get_cur_proc().unwrap();
    let unused_proc_ctx = &mut ProcContext::empty() as *mut ProcContext;
    let cur_proc_ctx: *const ProcContext = cur_proc.exclusive_access().trap_ctx_ppn.get_mut();
    unsafe {
        __switch(unused_proc_ctx, cur_proc_ctx);
    }
}
