#![no_std]
#![feature(linkage)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
extern crate alloc;

#[macro_use]
pub mod console;
mod lang_items;
mod logging;
pub mod syscall;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use bitflags::bitflags;
// use buddy_system_allocator::LockedHeap;
use buddy::Heap;

const USER_HEAP_SIZE: usize = 16 * 1024;
static mut USER_HEAP_SPACE: [u8; USER_HEAP_SIZE] = [0; USER_HEAP_SIZE];
#[global_allocator]
static USER_HEAP: Heap = Heap::new();
// static HEAP: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start(argc: usize, argv: usize) -> ! {
    init();

    // recover args
    let mut v: Vec<&'static str> = Vec::new();
    for i in 0..argc {
        let str_start =
            unsafe { ((argv + i * core::mem::size_of::<usize>()) as *const usize).read_volatile() };
        let len = (0usize..)
            .find(|i| unsafe { ((str_start + *i) as *const u8).read_volatile() == 0 })
            .unwrap();
        v.push(
            core::str::from_utf8(unsafe {
                core::slice::from_raw_parts(str_start as *const u8, len)
            })
            .unwrap(),
        );
    }

    exit(main(argc, v.as_slice()));
    // panic!("unreachable after sys_exit!");
}

#[linkage = "weak"]
#[no_mangle]
pub fn main(_argc: usize, _argv: &[&str]) -> i32 {
    panic!("Cannot find main!");
}

fn init() {
    unsafe {
        USER_HEAP
            .borrow_mut()
            .init(USER_HEAP_SPACE.as_ptr() as usize, USER_HEAP_SIZE);
        // HEAP.lock().init(USER_HEAP_SPACE.as_ptr() as usize, USER_HEAP_SIZE);
    }
    logging::init();
}

use syscall::*;
bitflags! {
    pub struct OpenFlags: u32 {
        const RDONLY = 0;
        const WRONLY = 1;
        const RDWR = 1 << 1;
        const CREATE = 1 << 9;
        const TRUNC = 1 << 10;
    }
}
pub fn dup(fd: usize) -> isize {
    sys_dup(fd)
}
pub fn open(path: &str, flags: OpenFlags) -> isize {
    sys_open(path, flags.bits)
}
pub fn close(fd: usize) -> isize {
    sys_close(fd)
}
pub fn read(fd: usize, buf: &mut [u8]) -> isize {
    sys_read(fd, buf)
}

pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}
pub fn exit(exit_code: i32) -> ! {
    sys_exit(exit_code)
}
pub fn yield_() -> isize {
    sys_yield()
}
pub fn get_time() -> isize {
    sys_get_time()
}
pub fn getpid() -> isize {
    sys_getpid()
}
pub fn fork() -> isize {
    sys_fork()
}
pub fn exec(path: &str, args: &[*const u8]) -> isize {
    sys_exec(path, args)
}
pub fn wait(exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(-1, exit_code as *mut _) {
            -2 => {
                yield_();
            }
            exit_pid => {
                return exit_pid;
            }
        }
    }
}
pub fn waitpid(pid: usize, exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(pid as isize, exit_code as *mut _) {
            -2 => {
                yield_();
            }
            exit_pid => {
                return exit_pid;
            }
        }
    }
}
pub fn waitpid_nb(pid: usize, exit_code: &mut i32) -> isize {
    sys_waitpid(pid as isize, exit_code as *mut _)
}
pub fn sleep(time_ms: usize) {
    sys_sleep(time_ms);
}

pub fn pipe(pipe_fd: &mut [usize]) -> isize {
    sys_pipe(pipe_fd)
}

/// Action for a signal
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct SignalAction {
    pub handler: usize,
    pub mask: SignalFlags,
}

impl Default for SignalAction {
    fn default() -> Self {
        Self {
            handler: 0,
            mask: SignalFlags::empty(),
        }
    }
}

pub const SIGDEF: i32 = 0; // Default signal handling
pub const SIGHUP: i32 = 1;
pub const SIGINT: i32 = 2;
pub const SIGQUIT: i32 = 3;
pub const SIGILL: i32 = 4;
pub const SIGTRAP: i32 = 5;
pub const SIGABRT: i32 = 6;
pub const SIGBUS: i32 = 7;
pub const SIGFPE: i32 = 8;
pub const SIGKILL: i32 = 9;
pub const SIGUSR1: i32 = 10;
pub const SIGSEGV: i32 = 11;
pub const SIGUSR2: i32 = 12;
pub const SIGPIPE: i32 = 13;
pub const SIGALRM: i32 = 14;
pub const SIGTERM: i32 = 15;
pub const SIGSTKFLT: i32 = 16;
pub const SIGCHLD: i32 = 17;
pub const SIGCONT: i32 = 18;
pub const SIGSTOP: i32 = 19;
pub const SIGTSTP: i32 = 20;
pub const SIGTTIN: i32 = 21;
pub const SIGTTOU: i32 = 22;
pub const SIGURG: i32 = 23;
pub const SIGXCPU: i32 = 24;
pub const SIGXFSZ: i32 = 25;
pub const SIGVTALRM: i32 = 26;
pub const SIGPROF: i32 = 27;
pub const SIGWINCH: i32 = 28;
pub const SIGIO: i32 = 29;
pub const SIGPWR: i32 = 30;
pub const SIGSYS: i32 = 31;

bitflags! {
    pub struct SignalFlags: i32 {
        const SIGDEF = 1; // Default signal handling
        const SIGHUP = 1 << 1;
        const SIGINT = 1 << 2;
        const SIGQUIT = 1 << 3;
        const SIGILL = 1 << 4;
        const SIGTRAP = 1 << 5;
        const SIGABRT = 1 << 6;
        const SIGBUS = 1 << 7;
        const SIGFPE = 1 << 8;
        const SIGKILL = 1 << 9;
        const SIGUSR1 = 1 << 10;
        const SIGSEGV = 1 << 11;
        const SIGUSR2 = 1 << 12;
        const SIGPIPE = 1 << 13;
        const SIGALRM = 1 << 14;
        const SIGTERM = 1 << 15;
        const SIGSTKFLT = 1 << 16;
        const SIGCHLD = 1 << 17;
        const SIGCONT = 1 << 18;
        const SIGSTOP = 1 << 19;
        const SIGTSTP = 1 << 20;
        const SIGTTIN = 1 << 21;
        const SIGTTOU = 1 << 22;
        const SIGURG = 1 << 23;
        const SIGXCPU = 1 << 24;
        const SIGXFSZ = 1 << 25;
        const SIGVTALRM = 1 << 26;
        const SIGPROF = 1 << 27;
        const SIGWINCH = 1 << 28;
        const SIGIO = 1 << 29;
        const SIGPWR = 1 << 30;
        const SIGSYS = 1 << 31;
    }
}

/// Send a signal to some process from current process.
pub fn kill(pid: usize, signum: i32) -> isize {
    sys_kill(pid, signum)
}

pub fn sigaction(
    signum: i32,
    action: Option<&SignalAction>,
    old_action: Option<&mut SignalAction>,
) -> isize {
    sys_sigaction(
        signum,
        action.map_or(core::ptr::null(), |a| a),
        old_action.map_or(core::ptr::null_mut(), |a| a),
    )
}

pub fn sigprocmask(mask: u32) -> isize {
    sys_sigprocmask(mask)
}

pub fn sigreturn() -> isize {
    sys_sigreturn()
}

pub fn thread_create(entry: usize, arg: usize) -> isize {
    sys_thread_create(entry, arg)
}

pub fn waittid(tid: usize) -> isize {
    loop {
        match sys_waittid(tid) {
            -2 => {
                yield_();
            }
            exit_code => return exit_code,
        }
    }
}
pub fn spin_mutex_create() -> isize {
    sys_mutex_create(false)
}
pub fn blocked_mutex_create() -> isize {
    sys_mutex_create(true)
}
pub fn mutex_lock(mutex_id: usize) {
    sys_mutex_lock(mutex_id);
}
pub fn mutex_unlock(mutex_id: usize) {
    sys_mutex_unlock(mutex_id);
}
pub fn condvar_create() -> isize {
    sys_condvar_create()
}
pub fn condvar_signal(condvar_id: usize) {
    sys_condvar_signal(condvar_id);
}
pub fn condvar_wait(condvar_id: usize, mutex_id: usize) {
    sys_condvar_wait(condvar_id, mutex_id);
}


pub const MAX_NAME_LENGTH: usize = 27;
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DirEntry {
    pub name: [u8; MAX_NAME_LENGTH + 1],
    pub inode_id: u32,
}
impl DirEntry {
    pub fn is_empty(&self) -> bool {
        self.inode_id == 0 && self.name.iter().all(|&c| c == 0)
    }
}

pub fn fstat(fd: usize) -> isize {
    sys_fstat(fd)
}
pub fn cd(path: &str) -> isize {
    // println!("cd {}", path);
    sys_cd(path)
}
pub fn cp(src: &str, dst: &str) -> isize {
    // println!("cp {} {}", src, dst);
    sys_cp(src, dst)
}
pub fn mv(src: &str, dst: &str) -> isize {
    // println!("mv {} {}", src, dst);
    sys_mv(src, dst)
}
pub fn rm(path: &str) -> isize {
    // println!("rm {}", path);
    sys_rm(path)
}

pub fn getcwd() -> String {
    let mut buf = [0u8; 1024];
    let len = sys_getcwd(&mut buf);
    assert!(len >= 0, "getcwd failed with error code: {}", len);
    let len = len as usize;
    let str = String::from_utf8_lossy(&buf[..len]).parse().unwrap();
    str
}
pub fn get_abs_path(path: &str) -> String {
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("{}{}", getcwd(), path)
    }
}

pub fn get_env_var_path() -> Vec<String> {
    Vec::from(["/bin/", "/tests/"])
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}

/// First try in cwd, then in environment variable PATH
pub fn get_exe_path(path: &str) -> Option<String> {
    if path.starts_with('/') {
        Some(path.to_string())
    } else {
        // try in cwd
        let abs_path = get_abs_path(path);
        let fd = open(&abs_path, OpenFlags::RDONLY);
        if fd >= 0 {
            close(fd as usize);
            return Some(abs_path);
        }
        // try in environment variable PATH
        for env_path in get_env_var_path() {
            let fd = open(&format!("{}{}", env_path, path), OpenFlags::RDONLY);
            if fd >= 0 {
                close(fd as usize);
                return Some(format!("{}{}", env_path, path));
            }
        }
        None
    }
}
#[macro_export]
macro_rules! vstore {
    ($var: expr, $value: expr) => {
        // unsafe { core::intrinsics::volatile_store($var_ref as *const _ as _, $value) }
        unsafe { core::ptr::write_volatile(core::ptr::addr_of_mut!($var), $value); }
    };
}

#[macro_export]
macro_rules! vload {
    ($var: expr) => {
        // unsafe { core::intrinsics::volatile_load($var_ref as *const _ as _) }
        unsafe { core::ptr::read_volatile(core::ptr::addr_of!($var)) }
    };
}

#[macro_export]
macro_rules! memory_fence {
    () => {
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst)
    };
}
