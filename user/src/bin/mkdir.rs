#![no_std]
#![no_main]

extern crate alloc;
extern crate user_lib;

use user_lib::{close, get_abs_path, open, OpenFlags};

#[no_mangle]
pub fn main(argc: usize, argv: &[&str]) -> i32 {
    assert!(argc == 2);
    let mut path = get_abs_path(argv[1]);
    if !path.ends_with("/") {
        path.push('/');
    }
    let fd = open(path.as_str(), OpenFlags::CREATE);
    if fd <= 0 {
        panic!("Error occured when opening file");
    }
    close(fd as usize);
    0
}
