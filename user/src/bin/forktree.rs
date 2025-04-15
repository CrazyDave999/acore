#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{exit, fork, getpid, sleep, yield_};

const DEPTH: usize = 4;

fn fork_child(cur: &str, branch: char) {
    let mut next = [0u8; DEPTH + 1];
    let l = cur.len();
    if l >= DEPTH {
        return;
    }
    next[..l].copy_from_slice(cur.as_bytes());
    next[l] = branch as u8;
    // println!("pid{}: haha", getpid());
    if fork() == 0 {
        fork_tree(core::str::from_utf8(&next[..l + 1]).unwrap());
        yield_();
        exit(0);
    }
}

fn fork_tree(cur: &str) {
    println!("pid{}: cur: {:#x}", getpid(), cur.as_ptr() as usize);
    println!("pid{}: cur: {}", getpid(), cur);
    fork_child(cur, '0');
    fork_child(cur, '1');
}

#[no_mangle]
pub fn main() -> i32 {
    // println!("pid{}: {:#x}", getpid(), "".as_ptr() as usize);
    // println!("pid{}: {:#x}", getpid(), "".as_ptr() as usize);
    fork_tree(" ");
    sleep(3000);
    0
}
