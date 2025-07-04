#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use user_lib::{close, get_abs_path, open, read, OpenFlags};
use alloc::format;

#[no_mangle]
pub fn main(argc: usize, argv: &[&str]) -> i32 {
    assert_eq!(argc, 2);
    let path = get_abs_path(argv[1]);
    let fd = open(format!("{}\0", path).as_str(), OpenFlags::RDONLY);
    if fd == -1 {
        panic!("Error occured when opening file");
    }
    let fd = fd as usize;
    let mut buf = [0u8; 256];
    loop {
        let size = read(fd, &mut buf) as usize;
        if size == 0 {
            break;
        }
        match core::str::from_utf8(&buf[..size]) {
            Ok(str) => {
                print!("{}", str);
            }
            Err(_) => {
                panic!("Error parsing read data as UTF-8");
            }
        }
    }
    close(fd);
    0
}
