#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::exit;

#[no_mangle]
pub fn main() -> i32 {
    println!("Goodbye!");
    exit(0);
}
