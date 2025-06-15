//! File and filesystem-related syscalls
use crate::fs::kernel_file::{KernelFile, OpenFlags, CWD};
use crate::fs::pipe::make_pipe_pair;
use crate::mm::VirtAddr;
use crate::proc::get_cur_proc;
use crate::{print, println};
use alloc::vec::Vec;

/// write buf of length `len`  to a file with `fd`
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let cur_proc = get_cur_proc();
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
    let cur_proc = get_cur_proc();
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
    let cur_proc = get_cur_proc();
    let mut inner = cur_proc.exclusive_access();
    let path = inner.mm.read_str(VirtAddr::from(path as usize));
    // println!("sys_open: path = {}, flags = {}", path, flags);
    if let Some(file) = KernelFile::from_path(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        inner.fd_table.insert_file(file)
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    let cur_proc = get_cur_proc();
    let mut inner = cur_proc.exclusive_access();
    inner.fd_table.dealloc_fd(fd)
}

pub fn sys_pipe(pipe: *mut usize) -> isize {
    let cur_proc = get_cur_proc();
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

pub fn sys_dup(fd: usize) -> isize {
    let cur_proc = get_cur_proc();
    let mut inner = cur_proc.exclusive_access();
    if let Some(file) = inner.get_file(fd) {
        inner.fd_table.insert_file(file.clone())
    } else {
        -1
    }
}

pub fn sys_fstat(fd: usize) -> isize {
    let cur_proc = get_cur_proc();
    let inner = cur_proc.exclusive_access();
    if let Some(file) = inner.get_file(fd) {
        print!("{}", file.stat());
        0
    } else {
        -1
    }
}

/// Change current pwd. path should be an absolute path which points to a directory.
pub fn sys_cd(path: *const u8) -> isize {
    // read the path from the user space
    let cur_proc = get_cur_proc();
    let inner = cur_proc.exclusive_access();
    let path = inner.mm.read_str(VirtAddr::from(path as usize));

    if path.chars().last() != Some('/') {
        return -1;
    }

    // check if the dir exists
    if let Some(_) = KernelFile::from_path(path.as_str(), OpenFlags::RDONLY) {
        // change the current working directory
        let mut cwd = CWD.exclusive_access();
        *cwd = path;
        0
    } else {
        // directory does not exist
        -1
    }
}

/// Get current working directory, which is a string. The method is similar to sys_read
pub fn sys_getcwd(buf: *const u8, len: usize) -> isize {
    let pwd = CWD.exclusive_access();
    let pwd_str = pwd.as_str();
    if pwd_str.len() < len {
        let cur_proc = get_cur_proc();
        let inner = cur_proc.exclusive_access();
        let mut vec = Vec::new();
        vec.extend_from_slice(pwd_str.as_bytes());
        inner.mm.write(VirtAddr::from(buf as usize), vec.as_slice());
        vec.len() as isize
    } else {
        // buffer is not large enough
        -1
    }
}
