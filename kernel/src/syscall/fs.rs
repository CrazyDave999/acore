//! File and filesystem-related syscalls

use crate::console::stdin::getchar;
use crate::print;
use crate::proc::get_cur_proc;
use alloc::vec::Vec;

const FD_STDIN: usize = 0;
const FD_STDOUT: usize = 1;

/// write buf of length `len`  to a file with `fd`
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let cur_proc = get_cur_proc().unwrap();
            let inner = cur_proc.exclusive_access();
            let vec = inner.mm.read((buf as usize).into(), len);
            let str = core::str::from_utf8(vec.as_slice()).unwrap();
            print!("{}", str);
            len as isize
        }
        _ => {
            panic!("Unsupported fd in sys_write!");
        }
    }
}
pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDIN => {
            let mut data = Vec::new();
            for _ in 0..len {
                data.push(getchar());
            }
            let cur_proc = get_cur_proc().unwrap();
            let inner = cur_proc.exclusive_access();
            inner.mm.write((buf as usize).into(), &data);
            len as isize
        }
        _ => {
            panic!("Unsupported fd in sys_read!");
        }
    }
}
