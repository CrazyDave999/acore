use alloc::sync::Arc;
use acore_fs::Inode;
use crate::fs::File;
use crate::sync::UPSafeCell;

pub struct KernelFile {
    readable: bool,
    writable: bool,
    inner: UPSafeCell<KernelFileInner>,
}
pub struct KernelFileInner {
    offset: usize,
    inode: Arc<Inode>,
}
impl KernelFile {
    pub fn new(readable: bool, writable: bool, inode: Arc<Inode>) -> Self {
        Self {
            readable,
            writable,
            inner: unsafe { UPSafeCell::new(KernelFileInner { offset: 0, inode }) },
        }
    }
}

impl File for KernelFile {
    fn readable(&self) -> bool {
        self.readable
    }

    fn writable(&self) -> bool {
        self.writable
    }

    fn read(&self, buf: &mut [u8]) -> usize {
        let mut inner = self.inner.exclusive_access();
        let read_size = inner.inode.read_at(inner.offset, buf);
        inner.offset += read_size;
        read_size
    }

    fn write(&self, buf: &[u8]) -> usize {
        let mut inner = self.inner.exclusive_access();
        let write_size = inner.inode.write_at(inner.offset, buf);
        assert_eq!(write_size, buf.len());
        inner.offset += write_size;
        write_size
    }

    fn seek(&self, offset: usize) -> usize {
        let mut inner = self.inner.exclusive_access();
        inner.offset = offset;
        inner.offset
    }
}