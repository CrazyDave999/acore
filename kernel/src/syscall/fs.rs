//! File and filesystem-related syscalls

use alloc::vec::Vec;
use crate::print;
use crate::console::stdin::getchar;
use crate::proc::get_cur_proc;

const FD_STDIN: usize = 0;
const FD_STDOUT: usize = 1;

/// write buf of length `len`  to a file with `fd`
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let slice = unsafe { core::slice::from_raw_parts(buf, len) };
            let str = core::str::from_utf8(slice).unwrap();
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
            get_cur_proc().unwrap().exclusive_access().mm.write(
                (buf as usize).into(),
                &data,
            );
            len as isize
        }
        _ => {
            panic!("Unsupported fd in sys_read!");
        }
    }
}
