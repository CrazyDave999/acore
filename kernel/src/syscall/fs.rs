//! File and filesystem-related syscalls

use alloc::string::String;
use alloc::sync::Arc;
use crate::fs::kernel_file::{KernelFile, OpenFlags, CWD, ROOT};
use crate::fs::pipe::make_pipe_pair;
use crate::fs::File;
use crate::mm::VirtAddr;
use crate::print;
use crate::proc::get_cur_proc;
use alloc::vec::Vec;
use acore_fs::Inode;

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

/// Remove redundant `.` and `..` in the path
pub fn simplify_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    let mut simplified: Vec<&str> = Vec::new();

    for part in parts {
        if part == "." {
            continue;
        } else if part == ".." {
            if simplified.len() > 1 {
                simplified.pop();
            }
        } else {
            simplified.push(part);
        }
    }
    simplified.join("/")
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
        *cwd = simplify_path(&path);
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

pub fn sys_cp(src: *const u8, dst: *const u8) -> isize {
    let cur_proc = get_cur_proc();
    let inner = cur_proc.exclusive_access();

    let src_path = inner.mm.read_str(VirtAddr::from(src as usize));
    let dst_path = inner.mm.read_str(VirtAddr::from(dst as usize));

    if src_path == dst_path {
        // print!("cp: cannot copy '{}' to itself\n", src_path);
        return -1;
    }

    if let Some(src_file) = KernelFile::from_path(src_path.as_str(), OpenFlags::RDONLY) {
        if let Some(dst_file) =
            KernelFile::from_path(dst_path.as_str(), OpenFlags::CREATE | OpenFlags::WRONLY)
        {
            let data = src_file.read_all();
            let write_size = dst_file.write(data.as_slice());
            if write_size == data.len() {
                // Successfully copied
                0
            } else {
                // print!("Error occurred when writing to destination file\n");
                -1
            }
        } else {
            // print!("Error occurred when opening destination file\n");
            -1
        }
    } else {
        // print!("Error occurred when opening source file\n");
        -1
    }
}

/// Only move the dir entry
pub fn sys_mv(src: *const u8, dst: *const u8) -> isize {
    let cur_proc = get_cur_proc();
    let inner = cur_proc.exclusive_access();

    let src_path = inner.mm.read_str(VirtAddr::from(src as usize));
    let dst_path = inner.mm.read_str(VirtAddr::from(dst as usize));

    if src_path == dst_path {
        // print!("mv: cannot move '{}' to itself\n", src_path);
        return -1;
    }
    let mut src_path = src_path.split('/').skip(1).collect::<Vec<_>>();
    let mut dst_path = dst_path.split('/').skip(1).collect::<Vec<_>>();
    let src_file_name = src_path.pop().unwrap();
    if src_file_name.is_empty() {
        // print!("mv: cannot move directory\n");
        return -1;
    }
    let dst_file_name = {
        let s = dst_path.pop().unwrap();
        if s.is_empty() {
            src_file_name
        } else {
            s
        }
    };
    let get_dir_inode = |path: Vec<&str>| -> Option<Arc<Inode>> {
        let mut inode = ROOT.clone();
        for dir_entry in path.iter() {
            inode = inode.access_dir_entry(dir_entry, acore_fs::DiskInodeType::Directory, false)?;
        }
        Some(inode)
    };
    if let Some(src_dir) = get_dir_inode(src_path) {
        if let Some(dst_dir) = get_dir_inode(dst_path) {
            if let Some(dst_file) = dst_dir.access_dir_entry(dst_file_name,
                                                             acore_fs::DiskInodeType::File, false) {
                // dst_file still exists, remove it first
                dst_file.clear();
                dst_dir.remove_dir_entry(dst_file_name);
            }
            // Move the file by changing its directory entry
            if let Some(inode_id) = src_dir.remove_dir_entry(src_file_name) {
                let mut fs = src_dir.fs.lock();
                dst_dir.insert_dir_entry(dst_file_name, inode_id, &mut fs);
            }
            0
        } else {
            // print!("Error occurred when accessing destination directory\n");
            -1
        }
    } else {
        // print!("Error occurred when accessing source directory\n");
        -1
    }
}

/// If is a dir, only remove when it is empty.
pub fn sys_rm(path: *const u8) -> isize {
    0
}