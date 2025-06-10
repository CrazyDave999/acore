//! File and filesystem-related syscalls
use crate::fs::kernel_file::{KernelFile, OpenFlags};
use crate::fs::pipe::make_pipe_pair;
use crate::mm::VirtAddr;
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

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    let cur_proc = get_cur_proc().unwrap();
    let mut inner = cur_proc.exclusive_access();
    let path = inner.mm.read_str(VirtAddr::from(path as usize));
    if let Some(file) = KernelFile::from_path(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        inner.fd_table.insert_file(file)
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    let cur_proc = get_cur_proc().unwrap();
    let mut inner = cur_proc.exclusive_access();
    inner.fd_table.dealloc_fd(fd)
}

pub fn sys_pipe(pipe: *mut usize) -> isize {
    let cur_proc = get_cur_proc().unwrap();
    let mut inner = cur_proc.exclusive_access();
    let (pipe_read, pipe_write) = make_pipe_pair();
    let read_fd = inner.fd_table.insert_file(pipe_read);
    let write_fd = inner.fd_table.insert_file(pipe_write);

    // write back the file descriptors to the pipe ptr
    let data = &[read_fd as usize, write_fd as usize];
    let byte_len = data.len() * core::mem::size_of::<usize>();
    inner.mm.write(VirtAddr::from(pipe as usize), unsafe {
        core::slice::from_raw_parts(data.as_ptr() as *const u8, byte_len)
    });
    0
}
