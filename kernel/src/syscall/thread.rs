use crate::proc::{get_cur_thread, push_thread, ThreadControlBlock};
use crate::trap::TrapContext;
use alloc::sync::Arc;

/// Create a new thread in the current process.
/// entry: the entry point of the thread function
/// arg: the argument to pass to the thread function
/// return the thread's TID
/// A group of thread resources will be allocated: user stack, trap context and kernel stack, etc.
/// No need to create new addr space, which is different from process creation.
pub fn sys_thread_create(entry: usize, arg: usize) -> isize {
    let thread = get_cur_thread().unwrap();
    let proc = thread.pcb.upgrade().unwrap();
    // create a new thread
    let new_thr = Arc::new(ThreadControlBlock::new(
        Arc::clone(&proc),
        thread
            .exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .user_stack_base,
        true,
    ));

    // push new thread to the scheduler
    push_thread(new_thr.clone());

    let new_thr_inner = new_thr.exclusive_access();
    let new_thr_res = new_thr_inner.res.as_ref().unwrap();
    let new_thr_tid = new_thr_res.tid;
    let mut proc_inner = proc.exclusive_access();

    // add new thread to the process
    while proc_inner.threads.len() < new_thr_tid + 1 {
        proc_inner.threads.push(None);
    }
    proc_inner.threads[new_thr_tid] = Some(Arc::clone(&new_thr));
    let new_thr_trap_ctx = new_thr_inner.get_trap_ctx();
    *new_thr_trap_ctx = TrapContext::app_init_context(
        entry,
        new_thr_res.get_user_stack_top(),
        proc_inner.mm.page_table.token(),
        new_thr.kernel_stack.get_top(),
    );
    (*new_thr_trap_ctx).x[10] = arg;
    new_thr_tid as isize
}
pub fn sys_gettid() -> isize {
    get_cur_thread().unwrap().exclusive_access().res.as_ref().unwrap().tid as isize
}

/// Wait for a thread to exit
/// thread does not exist, return -1
/// thread has not exited yet, return -2
/// otherwise, return thread's exit code
pub fn sys_waittid(tid: usize) -> i32 {
    let thread = get_cur_thread().unwrap();
    let proc = thread.pcb.upgrade().unwrap();
    let thr_inner = thread.exclusive_access();
    let mut proc_inner = proc.exclusive_access();

    // a thread cannot wait for itself
    if tid == thr_inner.res.as_ref().unwrap().tid {
        return -1;
    }

    let mut exit_code: Option<i32> = None;
    let target_thread = proc_inner.threads[tid].as_ref();
    if let Some(target_thread) = target_thread {
        if let Some(target_exit_code) = target_thread.exclusive_access().exit_code {
            exit_code = Some(target_exit_code);
        }
    } else {
        return -1;
    }
    if let Some(exit_code) = exit_code {
        proc_inner.threads[tid] = None;
        exit_code
    } else {
        -2
    }
}
