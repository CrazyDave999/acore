use alloc::sync::Arc;
use crate::mm::PageTable;
use crate::console::shutdown;
use crate::mm::get_app_data_by_name;
use crate::println;
use crate::proc::{get_cur_proc, get_cur_user_token, push_proc,switch_proc,exit_proc};
use crate::timer::get_time;
use crate::trap::TrapContext;

pub fn sys_exit(exit_code: i32) -> ! {
    exit_proc(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    switch_proc();
    0
}

pub fn sys_get_time() -> isize {
    get_time() as isize
}
pub fn sys_getpid() -> isize {
    get_cur_proc().unwrap().pid.0 as isize
}
pub fn sys_fork() -> isize {
    let cur_proc = get_cur_proc().unwrap();
    let new_proc = cur_proc.fork();
    let new_pid = new_proc.pid.0;
    let trap_ctx:&mut TrapContext = new_proc.exclusive_access().trap_ctx_ppn.get_mut();
    // for child process, fork returns 0. modify x[10] manually.
    trap_ctx.x[10] = 0;
    // add the new proc to the ready queue.
    push_proc(new_proc);
    new_pid as isize
}
pub fn sys_exec(path: *const u8) -> isize {
    let token = get_cur_user_token();
    let page_table = PageTable::from_token(token);
    let path = page_table.find_str((path as usize).into());
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        get_cur_proc().unwrap().exec(data);
        0
    } else {
        -1
    }
}
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let cur_proc = get_cur_proc().unwrap();

    let mut inner = cur_proc.exclusive_access();
    if !inner.children.iter().any(|p|pid==-1||pid as usize == p.getpid()) {
        // no such children, what the hell?
        return -1;
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| p.is_zombie() && (pid == -1 || pid as usize == p.getpid()));
    if let Some((ind, _)) = pair {
        let child = inner.children.remove(ind);
        // the resource this proc holds should be able to released
        assert_eq!(Arc::strong_count(&child), 1);
        let pid = child.getpid();
        let exit_code = child.exclusive_access().exit_code;

        inner.mm.write((exit_code_ptr as usize).into(), &exit_code.to_ne_bytes());
        pid as isize
    } else {
        -2
    }
}
