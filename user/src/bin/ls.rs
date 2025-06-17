#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern crate alloc;

use alloc::format;
use alloc::vec::Vec;
use user_lib::{close, get_abs_path, getcwd, open, read, DirEntry, OpenFlags};

#[no_mangle]
pub fn main(argc: usize, argv: &[&str]) -> i32 {
    assert!(argc <= 2, "Usage: ls [path]");
    let path = if argc == 1 {
        getcwd()
    } else {
        let mut s = get_abs_path(argv[1]);
        if !s.ends_with('/') {
            s.push('/');
        }
        s
    };
    let fd = open(format!("{}\0", &path).as_str(), OpenFlags::RDONLY);
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
        .iter()
        .filter_map(|&entry| if entry.is_empty() { None } else { Some(entry) })
        .collect()
    };
    let mut has_unhidden = false;
    for entry in dir_entries.iter() {
        let name = unsafe {
            core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                entry.name.as_ptr(),
                entry.name.len(),
            ))
        };
        if !name.starts_with(".") {
            print!("{}  ", name);
            has_unhidden = true;
        }
    }
    if has_unhidden {
        print!("\n");
    }
    close(fd);
    0
}
