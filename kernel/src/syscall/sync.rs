use crate::proc::{block_thread, get_cur_proc, get_cur_thread};
use crate::sync::{BlockedMutex, Condvar, Mutex, SpinMutex};
use crate::timer::{create_timer, get_time_ms};
use alloc::sync::Arc;

pub fn sys_sleep(ms: usize) -> isize {
    let expire_ms = get_time_ms() + ms;
    let thread = get_cur_thread().unwrap();
    create_timer(expire_ms, thread);
    block_thread();
    0
}

/// Return the mutex id (mid)
pub fn sys_mutex_create(is_blocked: bool) -> isize {
    let proc = get_cur_proc();
    let mutex: Option<Arc<dyn Mutex>> = if !is_blocked {
        Some(Arc::new(SpinMutex::new()))
    } else {
        Some(Arc::new(BlockedMutex::new()))
    };
    let mut proc_inner = proc.exclusive_access();
    if let Some((mid, _)) = proc_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
    {
        proc_inner.mutex_list[mid] = mutex;
        mid as isize
    } else {
        proc_inner.mutex_list.push(mutex);
        (proc_inner.mutex_list.len() - 1) as isize
    }
}

pub fn sys_mutex_lock(mid: usize) -> isize {
    let proc = get_cur_proc();
    let proc_inner = proc.exclusive_access();
    let mutex = Arc::clone(proc_inner.mutex_list[mid].as_ref().unwrap());
    drop(proc_inner);
    drop(proc);
    mutex.lock();
    0
}

pub fn sys_mutex_unlock(mid: usize) -> isize {
    let proc = get_cur_proc();
    let proc_inner = proc.exclusive_access();
    let mutex = Arc::clone(proc_inner.mutex_list[mid].as_ref().unwrap());
    drop(proc_inner);
    drop(proc);
    mutex.unlock();
    0
}

pub fn sys_condvar_create() -> isize {
    let proc = get_cur_proc();
    let mut proc_inner = proc.exclusive_access();
    let cid = if let Some(cid) = proc_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        proc_inner.condvar_list[cid] = Some(Arc::new(Condvar::new()));
        cid
    } else {
        proc_inner.condvar_list.push(Some(Arc::new(Condvar::new())));
        proc_inner.condvar_list.len() - 1
    };
    cid as isize
}

pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    let proc = get_cur_proc();
    let proc_inner = proc.exclusive_access();
    let condvar = Arc::clone(proc_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(proc_inner);
    drop(proc);
    condvar.signal();
    0
}

pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    let proc = get_cur_proc();
    let proc_inner = proc.exclusive_access();
    let condvar = Arc::clone(proc_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(proc_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(proc_inner);
    drop(proc);
    condvar.wait(mutex);
    0
}