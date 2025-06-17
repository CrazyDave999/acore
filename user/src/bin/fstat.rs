#![no_std]
#![no_main]

extern crate alloc;
extern crate user_lib;

use alloc::format;
use user_lib::{fstat, get_abs_path, open, print, OpenFlags};

#[no_mangle]
pub fn main(argc: usize, argv: &[&str]) -> i32 {
    assert_eq!(argc, 2);
    let path = get_abs_path(argv[1]);
    let fd = open(format!("{}\0", path).as_str(), OpenFlags::RDONLY);
    if fd == -1 {
        panic!("Error occured when opening file");
    }
    fstat(fd as usize);
    print!("\n");
    0
}
