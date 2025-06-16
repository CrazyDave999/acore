#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate user_lib;

#[no_mangle]
pub fn main(argc: usize, argv: &[&str]) -> i32 {
    assert_eq!(argc, 2);
    print!("{}", argv[1]);
    0
}
