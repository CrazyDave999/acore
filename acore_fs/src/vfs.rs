use spin::Mutex;
use alloc::sync::Arc;
use crate::afs::AcoreFileSystem;
use crate::block_cache::BlockCache;
use crate::block_dev::BlockDevice;
use crate::block_manager::get_block_cache;
use crate::layout::DiskInode;

/// for sys service related to file system

pub struct Inode {
    block_id: usize,
    block_offset: usize,
    fs: Arc<Mutex<AcoreFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<AcoreFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }

    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        let disk_inode = self.get_disk_inode_ref();
        disk_inode.read_at(offset, buf, &self.block_device)
    }
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let disk_inode = self.get_disk_inode_mut();
        todo!()
    }
    fn get_disk_inode_ref(&self) -> &DiskInode {
        let disk_inode = get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .as_ref::<DiskInode>(self.block_offset);
        disk_inode
    }
    fn get_disk_inode_mut(&self) -> &mut DiskInode {
        let disk_inode = get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .as_mut_ref::<DiskInode>(self.block_offset);
        disk_inode
    }

    /// List all inodes under current inode
    pub fn ls(&self) -> Vec<String> {
        let _fs = self.fs.lock();
        todo!()
    }
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        let disk_inode = self.get_disk_inode_mut();
        let size = disk_inode.size;

    }
}