use alloc::string::String;

pub mod kernel_file;
pub mod stdio;
pub mod pipe;


pub trait File: Send + Sync {
    fn readable(&self) -> bool;
    fn writable(&self) -> bool;
    fn read(&self, buf: &mut [u8]) -> usize;
    fn write(&self, buf: &[u8]) -> usize;
    #[allow(unused)]
    fn seek(&self, offset: usize) -> usize;
    #[allow(unused)]
    fn stat(&self) -> String;
}
