use super::pcb::{ProcessControlBlock, ProcessState};
use alloc::collections::BTreeMap;

use crate::proc::scheduler::Scheduler;

use crate::console::shutdown;
use crate::println;
use crate::proc::ctx::ThreadContext;
use crate::proc::switch::__switch;
use crate::proc::thread::{ThreadControlBlock, ThreadState};
use crate::proc::INIT_PCB;
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;

pub struct ThreadManager {
    cur: Option<Arc<ThreadControlBlock>>,
    scheduler: Scheduler,
    pid2pcb: BTreeMap<usize, Arc<ProcessControlBlock>>,
}

impl ThreadManager {
    pub fn new() -> Self {
        ThreadManager {
            cur: None,
            // procs: Vec::new(),
            scheduler: Scheduler::new(),
            pid2pcb: BTreeMap::new(),
        }
    }
    pub fn current(&self) -> Option<Arc<ThreadControlBlock>> {
        self.cur.as_ref().map(Arc::clone)
    }
}

lazy_static! {
    pub static ref THREAD_MANAGER: UPSafeCell<ThreadManager> =
        unsafe { UPSafeCell::new(ThreadManager::new()) };
}

pub fn get_cur_thread() -> Option<Arc<ThreadControlBlock>> {
    THREAD_MANAGER.exclusive_access().current()
}
pub fn get_cur_proc() -> Arc<ProcessControlBlock> {
    // println!("[kernel] get_cur_proc");
    get_cur_thread().unwrap().pcb.upgrade().unwrap()
}
/// Get current process's root_ppn of the page table
pub fn get_cur_user_token() -> usize {
    // println!("[kernel] get_cur_user_token");
    get_cur_proc().token()
}

/// Get mutable reference to current thread's trap context
pub fn get_cur_trap_ctx() -> &'static mut TrapContext {
    get_cur_thread().unwrap().exclusive_access().get_trap_ctx()
}

/// Suspend current process and switch to a ready one
pub fn switch_thread() {
    // println!("[kernel] switch_proc: pid: {:?}", get_cur_proc().unwrap().getpid());
    let mut inner = THREAD_MANAGER.exclusive_access();
    if let Some(next_thread) = inner.scheduler.pop() {
        // println!("1");
        if let Some(cur_thread) = inner.cur.clone() {
            // println!(
            //     "2 cur_pid: {:?}, next_pid: {:?}",
            //     cur_proc.getpid(),
            //     next_proc.getpid()
            // );
            let mut next_inner = next_thread.exclusive_access();
            next_inner.state = ThreadState::Running;
            let next_thr_ctx: *mut ThreadContext = &mut next_inner.thread_ctx as *mut _;

            let mut cur_inner = cur_thread.exclusive_access();
            cur_inner.state = ThreadState::Ready;
            let cur_thr_ctx: *mut ThreadContext = &mut cur_inner.thread_ctx as *mut _;

            drop(next_inner);
            drop(cur_inner);
            inner.cur = Some(next_thread);
            inner.scheduler.push(cur_thread);
            drop(inner);
            // println!("going to switch, cur_proc_ctx_ptr: {:#x}, next_proc_ctx_ptr: {:#x}",
            //          cur_proc_ctx
            //     as usize,
            //          next_proc_ctx as usize);
            // unsafe {
            //     println!("cur_proc_ctx: {:?}", cur_proc_ctx.as_ref().unwrap());
            //     println!("next_proc_ctx: {:?}", next_proc_ctx.as_ref().unwrap());
            // }
            unsafe {
                __switch(cur_thr_ctx, next_thr_ctx);
            }
            return;
        } else {
            // println!("3 next_pid: {:?}", next_proc.getpid());
            // no current process, just switch to next
            let mut next_inner = next_thread.exclusive_access();
            next_inner.state = ThreadState::Running;
            let next_thr_ctx = &next_inner.thread_ctx as *const _;

            let unused_thr_ctx = &mut ThreadContext::empty() as *mut _;

            drop(next_inner);
            inner.cur = Some(next_thread);
            drop(inner);
            // println!("going to switch");
            unsafe {
                __switch(unused_thr_ctx, next_thr_ctx);
            }
            return;
        }
    }
    // no other ready thread, do nothing
}

/// Exit current proc and switch to a ready one. If the current proc is init, then shutdown.
pub fn exit_thread(exit_code: i32) {
    // println!("[kernel] exit_proc");
    let cur_thr = get_cur_thread().unwrap();
    let mut cur_thr_inner = cur_thr.exclusive_access();
    let cur_proc = cur_thr.pcb.upgrade().unwrap();
    let tid = cur_thr_inner.res.as_ref().unwrap().tid;

    // record exit code
    cur_thr_inner.exit_code = Some(exit_code);

    // clear cur thread's resources
    cur_thr_inner.res = None;

    drop(cur_thr_inner);
    drop(cur_thr);

    if tid == 0 {
        // this is the main thread of the process
        let pid = cur_proc.getpid();
        if pid == INIT_PCB.getpid() {
            println!("[kernel] Goodbye! exit code: {}", exit_code);
            shutdown();
        }
        remove_from_pid2pcb(pid);
        let mut cur_proc_inner = cur_proc.exclusive_access();
        // mark current proc as a zombie, and its pcb will be recycled later
        cur_proc_inner.state = ProcessState::Zombie;
        cur_proc_inner.exit_code = exit_code;

        // move all children to init
        let mut init_inner = INIT_PCB.exclusive_access();
        for child in cur_proc_inner.children.iter() {
            child.exclusive_access().parent = Some(Arc::downgrade(&INIT_PCB));
            init_inner.children.push(Arc::clone(child));
        }
        drop(init_inner);

        // dealloc all threads' resources
        let mut recycle_resources = Vec::new();
        for thread in cur_proc_inner.threads.iter() {
            if thread.is_none() {
                continue;
            }
            let thread = thread.as_ref().unwrap();
            // if there are threads in scheduler's ready queue, we should remove them
            remove_thread(Arc::clone(thread));
            let mut thr_inner = thread.exclusive_access();
            if let Some(res) = thr_inner.res.take() {
                recycle_resources.push(res);
            }
        }
        // deallocating threads' resources require access to PCB inner, so we
        // need to collect those resources, then release cur_proc_inner
        // for now to avoid deadlock/double borrow problem.
        drop(cur_proc_inner);
        recycle_resources.clear();

        // require access to cur_proc_inner again
        let mut cur_proc_inner = cur_proc.exclusive_access();
        cur_proc_inner.children.clear();
        cur_proc_inner.mm.recycle_data_pages();
        cur_proc_inner.fd_table.clear();

        // remove all threads except the main thread, whose tcb will be deallocated in waitpid
        while cur_proc_inner.threads.len() > 1 {
            cur_proc_inner.threads.pop();
        }
    }

    drop(cur_proc);

    // now this thread's tcb still exists
    // give out control flow and never come back
    // waiting for parent to release all its resources. e.g. page table
    THREAD_MANAGER.exclusive_access().cur = None;
    switch_thread()
}

/// Push a newly created thread to the scheduler's ready queue.
pub fn push_thread(tcb: Arc<ThreadControlBlock>) {
    let mut inner = THREAD_MANAGER.exclusive_access();
    inner.scheduler.push(tcb);
}

/// Fetch a thread from the scheduler's ready queue and let it possess the cpu.
pub fn pop_thread() -> Option<Arc<ThreadControlBlock>> {
    let mut inner = THREAD_MANAGER.exclusive_access();
    inner.scheduler.pop()
}

pub fn remove_thread(tcb: Arc<ThreadControlBlock>) {
    let mut inner = THREAD_MANAGER.exclusive_access();
    inner.scheduler.remove(tcb);
}

pub fn pid2pcb(pid: usize) -> Option<Arc<ProcessControlBlock>> {
    let inner = THREAD_MANAGER.exclusive_access();
    inner.pid2pcb.get(&pid).cloned()
}

pub fn insert_to_pid2pcb(pid: usize, proc: Arc<ProcessControlBlock>) {
    if THREAD_MANAGER
        .exclusive_access()
        .pid2pcb
        .insert(pid, proc)
        .is_some()
    {
        panic!("insert_to_pid2pcb: pid {} already exists", pid);
    }
}

pub fn remove_from_pid2pcb(pid: usize) {
    if THREAD_MANAGER
        .exclusive_access()
        .pid2pcb
        .remove(&pid)
        .is_none()
    {
        panic!("remove_from_pid2pcb: pid {} not found", pid);
    }
}

pub fn launch(proc: Arc<ProcessControlBlock>) {
    let thread = proc.exclusive_access().get_thread(0);
    let mut inner = THREAD_MANAGER.exclusive_access();
    inner.cur = Some(thread);
    inner.cur.as_ref().unwrap().exclusive_access().state = ThreadState::Running;
    drop(inner);
    let cur_thr = get_cur_thread().unwrap();
    let cur_thr_inner = cur_thr.exclusive_access();
    let unused_thr_ctx = &mut ThreadContext::empty() as *mut _;
    let cur_thr_ctx = &cur_thr_inner.thread_ctx as *const _;
    drop(cur_thr_inner);
    drop(cur_thr);
    unsafe {
        __switch(unused_thr_ctx, cur_thr_ctx);
    }
}
