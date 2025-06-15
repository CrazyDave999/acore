#![no_std]
#![no_main]

extern crate alloc;

extern crate user_lib;

use alloc::format;
use user_lib::{fstat, open, print, OpenFlags};

#[no_mangle]
pub fn main(argc: usize, argv: &[&str]) -> i32 {
    let pwd= "/";
    assert!(argc == 2);
    let fd = open(format!("{}{}", pwd, argv[1]).as_str(), OpenFlags::RDONLY);
    if fd == -1 {
        panic!("Error occured when opening file");
    }
    fstat(fd as usize);
    print!("\n");
    0
}
