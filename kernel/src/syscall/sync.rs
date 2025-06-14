use alloc::sync::Arc;
use crate::proc::{block_thread, get_cur_proc, get_cur_thread};
use crate::sync::{BlockedMutex, Mutex, SpinMutex};
use crate::timer::{create_timer, get_time_ms};

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