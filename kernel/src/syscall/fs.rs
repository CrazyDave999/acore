//! File and filesystem-related syscalls
use crate::proc::get_cur_proc;
use alloc::vec::Vec;


/// write buf of length `len`  to a file with `fd`
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let cur_proc = get_cur_proc().unwrap();
    let inner = cur_proc.exclusive_access();
    let vec = inner.mm.read((buf as usize).into(), len);
    if let Some(file) = inner.get_file(fd) {
        if !file.writable() {
            -1
        } else {
            let file = file.clone();
            drop(inner);
            file.write(vec.as_slice()) as isize
        }
    } else {
        -1
    }
}
pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let cur_proc = get_cur_proc().unwrap();
    let inner = cur_proc.exclusive_access();
    if let Some(file) = inner.get_file(fd) {
        if !file.readable() {
            -1
        } else {
            let mut vec = Vec::new();
            vec.resize(len, 0);
            let file = file.clone();
            drop(inner);
            let ret = file.read(vec.as_mut_slice());
            let inner = cur_proc.exclusive_access();
            inner.mm.write((buf as usize).into(), vec.as_slice());
            ret as isize
        }
    } else {
        -1
    }
}

pub fn sys_open(path: &str, flags: u32) -> isize {
    todo!()
}

pub fn sys_close(fd: usize) -> isize {
    todo!()
}