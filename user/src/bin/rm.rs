#![no_std]
#![no_main]

extern crate alloc;
extern crate user_lib;

use user_lib::{get_abs_path, println, rm};

#[no_mangle]
pub fn main(_argc: usize, argv: &[&str]) -> i32 {
    for arg in argv.iter().skip(1) {
        if rm(get_abs_path(*arg).as_str()) < 0 {
            println!("[rm] '{}' doesn't exist or is a non-empty directory", argv[1]);
            return -1;
        }
    }
    0
}
