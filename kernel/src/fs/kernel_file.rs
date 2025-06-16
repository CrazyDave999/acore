use crate::drivers::BLOCK_DEVICE;
use crate::fs::File;
use crate::sync::UPSafeCell;
use acore_fs::AcoreFileSystem;
use acore_fs::{DiskInodeType, Inode, BLOCK_SIZE};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use bitflags::bitflags;
use lazy_static::lazy_static;

bitflags! {
    ///Open file flags
    pub struct OpenFlags: u32 {
        ///Read only
        const RDONLY = 0;
        ///Write only
        const WRONLY = 1 << 0;
        ///Read & Write
        const RDWR = 1 << 1;
        ///Allow create
        const CREATE = 1 << 9;
        ///Clear file and return an empty one
        const TRUNC = 1 << 10;
    }
}

impl OpenFlags {
    /// Do not check validity for simplicity
    /// Return (readable, writable)
    pub fn read_write(&self) -> (bool, bool) {
        if self.is_empty() {
            (true, false)
        } else if self.contains(Self::WRONLY) {
            (false, true)
        } else {
            (true, true)
        }
    }
}

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

    /// Read all data from the kernel file into a vector
    pub fn read_all(&self) -> Vec<u8> {
        let mut inner = self.inner.exclusive_access();
        let mut buf = [0u8; BLOCK_SIZE];
        let mut v = Vec::new();
        loop {
            let len = inner.inode.read_at(inner.offset, &mut buf);
            if len == 0 {
                break;
            }
            inner.offset += len;
            v.extend_from_slice(&buf[..len]);
        }
        v
    }
    pub fn from_path(path: &str, flags: OpenFlags) -> Option<Arc<Self>> {
        // println!("from_path {:?}", path);
        let is_dir = path.ends_with('/');
        let mut path = path.split('/').skip(1).collect::<Vec<_>>();

        let file_name = path.pop().unwrap();
        let mut inode = ROOT.clone();
        if path.len() > 0 {
            for dir_entry in path[..path.len() - 1].iter() {
                inode = inode.access_dir_entry(*dir_entry, DiskInodeType::Directory, false)?;
            }
        }
        let (readable, writable) = flags.read_write();
        let create = if flags.contains(OpenFlags::CREATE) {
            true
        } else {
            false
        };
        if is_dir && create {
            assert!(file_name.is_empty());
            assert!(path.len() > 0);
            inode = inode.access_dir_entry(path[path.len() - 1], DiskInodeType::Directory, true)?;
        } else {
            if path.len() > 0 {
                inode = inode.access_dir_entry(
                    path[path.len() - 1],
                    DiskInodeType::Directory,
                    false,
                )?;
            }
            let type_ = if is_dir {
                assert!(file_name.is_empty());
                DiskInodeType::Directory
            } else {
                DiskInodeType::File
            };
            inode = inode.access_dir_entry(file_name, type_, create)?;
            if flags.contains(OpenFlags::TRUNC) {
                inode.clear();
            }
        }
        Some(Arc::new(Self::new(readable, writable, inode)))
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

    fn stat(&self) -> String {
        self.inner.exclusive_access().inode.fstat()
    }
}

lazy_static! {
    pub static ref ROOT: Arc<Inode> = {
        let afs = AcoreFileSystem::open(BLOCK_DEVICE.clone());
        AcoreFileSystem::root_inode(afs)
    };
    pub static ref CWD: UPSafeCell<String> = unsafe { UPSafeCell::new(String::from("/")) };
}

