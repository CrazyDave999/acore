#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use alloc::format;
use alloc::vec::Vec;
use user_lib::{close, fstat, getcwd, open, read, DirEntry, OpenFlags};

#[no_mangle]
pub fn main() -> i32 {
    let cwd = getcwd();
    let dir_fd = open(format!("{}\0", cwd).as_str(), OpenFlags::RDONLY);
    if dir_fd == -1 {
        panic!("Error occured when opening file");
    }
    let dir_fd = dir_fd as usize;
    let mut buf = [0u8; 256];
    let mut dir_data = Vec::new();
    loop {
        let size = read(dir_fd, &mut buf) as usize;
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
    println!("total {} entries:", dir_entries.len());
    for entry in dir_entries {
        if entry.is_empty() {
            continue;
        }
        let name = unsafe {
            let raw_name = &entry.name[..];
            let len = raw_name.iter().position(|&c| c == 0).unwrap_or(raw_name.len());
            core::str::from_utf8_unchecked(&raw_name[..len])
        };
        print!("{}", format!("Name: {:<30}", name));
        let fd = open(format!("{}{}\0", cwd, name).as_str(), OpenFlags::RDONLY);
        fstat(fd as usize);
        close(fd as usize);
        print!("\n");
    }
    // print!("\n");
    close(dir_fd);
    0
}
