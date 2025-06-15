#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern crate alloc;

use alloc::format;
use alloc::vec::Vec;
use user_lib::{close, getcwd, open, read, DirEntry, OpenFlags};

#[no_mangle]
pub fn main() -> i32 {
    let cwd = getcwd();
    let fd = open(format!("{}\0", cwd).as_str(), OpenFlags::RDONLY);
    if fd == -1 {
        panic!("Error occured when opening file");
    }
    let fd = fd as usize;
    let mut buf = [0u8; 256];
    let mut dir_data = Vec::new();
    loop {
        let size = read(fd, &mut buf) as usize;
        if size == 0 {
            break;
        }
        dir_data.extend_from_slice(&buf[..size]);
    }
    assert_eq!(
        dir_data.len() % core::mem::size_of::<DirEntry>(),
        0,
        "Directory data size is not a multiple of DirEntry size"
    );
    let dir_entries: Vec<DirEntry> = unsafe {
        core::slice::from_raw_parts(
            dir_data.as_ptr() as *const DirEntry,
            dir_data.len() / core::mem::size_of::<DirEntry>(),
        )
        .to_vec()
    };
    for entry in dir_entries {
        let name = unsafe {
            core::str::from_utf8_unchecked(
                core::slice::from_raw_parts(entry.name.as_ptr(), entry.name.len()),
            )
        };
        print!("{}  ", name);
    }
    print!("\n");
    close(fd);
    0
}
