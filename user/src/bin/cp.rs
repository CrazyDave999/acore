#![no_std]
#![no_main]

extern crate alloc;
extern crate user_lib;

use user_lib::{close, cp, get_abs_path, open, println, read, write, OpenFlags};
use alloc::format;

#[no_mangle]
pub fn main(argc: usize, argv: &[&str]) -> i32 {
    assert_eq!(argc, 3);
    let src = get_abs_path(argv[1]);
    let dst = get_abs_path(argv[2]);
    let ret = cp(src.as_str(), dst.as_str());
    if ret < 0 {
        println!("cp {} {} failed: {}", src, dst, ret);
        return -1;
    }
    0
}
