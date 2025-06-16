#![no_std]
#![no_main]

extern crate alloc;
extern crate user_lib;

use user_lib::{get_abs_path, mv, println};

#[no_mangle]
pub fn main(argc: usize, argv: &[&str]) -> i32 {
    assert_eq!(argc, 3);
    let src = get_abs_path(argv[1]);
    let dst = get_abs_path(argv[2]);
    if src == dst {
        println!("'{}' and '{}' are the same file", argv[1], argv[2]);
        return -1;
    }
    let ret = mv(src.as_str(), dst.as_str());
    if ret < 0 {
        println!("cp {} {} failed: {}", src, dst, ret);
        return -1;
    }
    0
}
