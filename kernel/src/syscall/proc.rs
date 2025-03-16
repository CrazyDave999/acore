use crate::console::shutdown;
use crate::println;
use crate::timer::get_time;
pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    shutdown()
}

pub fn sys_yield() -> isize {
    0
}

pub fn sys_get_time() -> isize {
    get_time() as isize
}