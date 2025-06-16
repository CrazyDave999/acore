#![no_std]
#![no_main]

extern crate alloc;

extern crate user_lib;

use user_lib::{close, get_abs_path, open, println};

#[no_mangle]
pub fn main(_argc: usize, argv: &[&str]) -> i32 {
    for arg in argv.iter().skip(1) {
        let fd = open(get_abs_path(*arg).as_str(), user_lib::OpenFlags::CREATE |
            user_lib::OpenFlags::WRONLY);
        if fd < 0 {
            println!("[touch] '{}' cannot be created", arg);
            return -1;
        }
        close(fd as usize);
    }
    0
}
