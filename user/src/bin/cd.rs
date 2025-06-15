#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use user_lib::{cd, get_abs_path};

#[no_mangle]
pub fn main(argc: usize, argv: &[&str]) -> i32 {
    assert_eq!(argc, 2);
    // println!("[user] cd: {}", argv[1]);
    let mut path = get_abs_path(argv[1]);
    if !path.ends_with("/") {
        path.push('/');
    }
    path.push('\0');
    cd(path.as_str());
    0
}
