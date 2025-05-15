use super::File;


pub struct Stdin;
pub struct Stdout;

use crate::console::stdin::getchar;
use crate::print;

impl File for Stdin {
    fn readable(&self) -> bool {
        true
    }
    fn writable(&self) -> bool {
        false
    }
    fn read(&self, buf: &mut [u8]) -> usize {
        for b in buf.iter_mut() {
            *b = getchar();
        }
        1
    }
    fn write(&self, buf: &[u8]) -> usize {
        panic!("WTF? Cannot write to stdin!");
    }
    /// do nothing
    fn seek(&self, _offset: usize) -> usize {
        0
    }
}
impl File for Stdout {
    fn readable(&self) -> bool {
        false
    }
    fn writable(&self) -> bool {
        true
    }
    fn read(&self, _buf: &mut [u8]) -> usize {
        panic!("WTF? Cannot read from stdout!");
    }
    fn write(&self, buf: &[u8]) -> usize {
        print!("{}", core::str::from_utf8(buf).unwrap());
        buf.len()
    }
    /// do nothing
    fn seek(&self, _offset: usize) -> usize {
        0
    }
}

