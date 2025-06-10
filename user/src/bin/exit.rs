#![no_std]
#![no_main]

extern crate user_lib;

use user_lib::syscall;

#[no_mangle]
pub fn main() -> i32 {
    syscall::sys_shutdown();
    0
}