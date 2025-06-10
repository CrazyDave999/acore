use crate::console::shutdown;
use crate::fs::kernel_file::{KernelFile, OpenFlags};
use crate::fs::File;
use crate::mm::{PageTable, VirtAddr};
use crate::proc::{exit_proc, get_cur_proc, get_cur_user_token, push_proc, switch_proc};
use crate::timer::get_time_ms;
use crate::trap::TrapContext;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

pub fn sys_exit(exit_code: i32) -> ! {
    // println!("[kernel] sys_exit: pid: {}", sys_getpid());
    exit_proc(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    // println!("[kernel] sys_yield: pid: {}", sys_getpid());
    switch_proc();
    // println!("[kernel] back from switch: pid: {}", sys_getpid());
    0
}

pub fn sys_get_time() -> isize {
    get_time_ms() as isize
}
pub fn sys_getpid() -> isize {
    get_cur_proc().unwrap().pid.0 as isize
}
pub fn sys_fork() -> isize {
    // println!("[kernel] sys_fork: pid: {}", sys_getpid());
    let cur_proc = get_cur_proc().unwrap();
    let new_proc = cur_proc.fork();
    let new_pid = new_proc.pid.0;
    let trap_ctx: &mut TrapContext = new_proc.exclusive_access().trap_ctx_ppn.get_mut();
    // for child process, fork returns 0. modify x[10] manually.
    trap_ctx.x[10] = 0;
    // add the new proc to the ready queue.
    push_proc(new_proc);
    new_pid as isize
}
pub fn sys_exec(path: *const u8, mut args: *const usize) -> isize {
    // println!("[kernel] sys_exec: pid: {}", sys_getpid());
    let token = get_cur_user_token();
    let page_table = PageTable::from_token(token);
    let path = page_table.find_str((path as usize).into());
    // println!("path: {}", path);

    // fetch args in user addr space
    let cur_proc = get_cur_proc().unwrap();
    let inner = cur_proc.exclusive_access();
    let mut args_vec: Vec<String> = Vec::new();
    loop {
        let args_pa = inner.mm.page_table.find_pa(VirtAddr::from(args as usize)).unwrap().0;
        unsafe {
            let arg_str_ptr = *(args_pa as *const usize);
            if arg_str_ptr == 0 {
                break;
            }
            args_vec.push(inner.mm.read_str(VirtAddr::from(arg_str_ptr)));
            args = args.add(1);
        }
    }
    drop(inner);

    if let Some(app_kernel_file) = KernelFile::from_path(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_kernel_file.read_all();
        let argc = args_vec.len();
        cur_proc.exec(all_data.as_slice(), args_vec);
        // return argc because cx.x[10] will be covered with it later
        argc as isize
    } else {
        -1
    }
}
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    // println!("[kernel] sys_waitpid: pid: {}", sys_getpid());
    let cur_proc = get_cur_proc().unwrap();

    let mut inner = cur_proc.exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        // no such children, what the hell?
        return -1;
    }
    let pair = inner
        .children
        .iter()
        .enumerate()
        .find(|(_, p)| p.is_zombie() && (pid == -1 || pid as usize == p.getpid()));
    if let Some((ind, _)) = pair {
        let child = inner.children.remove(ind);
        // the resource this proc holds should be able to released
        assert_eq!(Arc::strong_count(&child), 1);
        let pid = child.getpid();
        let exit_code = child.exclusive_access().exit_code;

        inner
            .mm
            .write((exit_code_ptr as usize).into(), &exit_code.to_ne_bytes());
        pid as isize
    } else {
        -2
    }
}

pub fn sys_shutdown() -> ! {
    shutdown();
}
