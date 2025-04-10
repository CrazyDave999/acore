use super::pcb::{ProcessControlBlock, ProcessState};

use crate::proc::scheduler::Scheduler;

use crate::proc::proc_ctx::ProcContext;
use crate::proc::switch::__switch;
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use alloc::sync::Arc;
use lazy_static::lazy_static;
use crate::console::shutdown;
use crate::println;
use crate::proc::INIT_PCB;

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
pub fn switch_proc() {
    let mut inner = PROC_MANAGER.exclusive_access();
    if let Some(next_proc) = inner.scheduler.pop() {
        if let Some(cur_proc) = get_cur_proc() {
            let mut next_inner = next_proc.exclusive_access();
            next_inner.state = ProcessState::Running;
            let next_proc_ctx= &next_inner.proc_ctx as *const _;

            let mut cur_inner = cur_proc.exclusive_access();
            cur_inner.state = ProcessState::Ready;
            let cur_proc_ctx = &mut cur_inner.proc_ctx as *mut _;

            drop(next_inner);
            drop(cur_inner);
            inner.cur = Some(next_proc);
            inner.scheduler.push(cur_proc);
            drop(inner);
            unsafe {
                __switch(cur_proc_ctx, next_proc_ctx)
            }
        } else {
            // no current process, just switch to next
            let mut next_inner = next_proc.exclusive_access();
            next_inner.state = ProcessState::Running;
            let next_proc_ctx= &next_inner.proc_ctx as *const _;

            let unused_proc_ctx = &mut ProcContext::empty() as *mut _;

            drop(next_inner);
            inner.cur = Some(next_proc);
            drop(inner);
            unsafe {
                __switch(unused_proc_ctx, next_proc_ctx)
            }
        }
    }
    // no other ready proc, do nothing
}

/// Exit current proc and switch to a ready one. If the current proc is init, then shutdown.
pub fn exit_proc(exit_code: i32) {
    let cur_proc = get_cur_proc().unwrap();
    let pid = cur_proc.getpid();
    if pid == INIT_PCB.getpid() {
        println!("[kernel] Goodbye! exit code: {}", exit_code);
        shutdown();
    }
    // cur_proc is not init
    let mut inner = cur_proc.exclusive_access();
    inner.state = ProcessState::Zombie;
    inner.exit_code = exit_code;

    let mut init_inner = INIT_PCB.exclusive_access();
    for child in inner.children.iter() {
        child.exclusive_access().parent = Some(Arc::downgrade(&INIT_PCB));
        init_inner.children.push(Arc::clone(child));
    }
    drop(init_inner);

    inner.children.clear();
    inner.mm.recycle_data_pages();
    drop(inner);
    drop(cur_proc);

    // now this proc's pcb still exists
    // give out control flow and never come back
    // waiting for parent to release all its resources. e.g. page table
    PROC_MANAGER.exclusive_access().cur = None;
    switch_proc()
}

/// Push a newly created process to the scheduler's ready queue.
pub fn push_proc(proc: Arc<ProcessControlBlock>) {
    &PROC_MANAGER.exclusive_access().scheduler.push(proc);
}

pub fn launch(proc: Arc<ProcessControlBlock>) -> ! {
    println!("launching proc: {:?}", proc.getpid());
    let mut inner = PROC_MANAGER.exclusive_access();
    inner.cur = Some(proc);
    inner.cur.as_ref().unwrap().set_state(ProcessState::Running);
    drop(inner);
    let cur_proc = get_cur_proc().unwrap();
    let unused_proc_ctx = &mut ProcContext::empty() as *mut ProcContext;
    let cur_proc_ctx: *const ProcContext = cur_proc.exclusive_access().trap_ctx_ppn.get_mut();
    println!("going to switch");
    unsafe {
        __switch(unused_proc_ctx, cur_proc_ctx);
    }
}
