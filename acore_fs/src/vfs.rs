use alloc::string::String;
use crate::afs::AcoreFileSystem;
use crate::block_dev::BlockDevice;
use crate::block_manager::{get_block_cache, sync_all};
use crate::layout::{DirEntry, DiskInode, DiskInodeType, DIR_ENTRY_SIZE};
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::{Mutex, MutexGuard};

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
    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<AcoreFileSystem>,
    ) {
        if new_size <= disk_inode.size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data_block());
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    }

    /// Find an inode by name in current directory
    pub fn access_dir_entry(
        &self,
        name: &str,
        type_: DiskInodeType,
        create: bool,
    ) -> Option<Arc<Inode>> {
        let cache = get_block_cache(self.block_id, Arc::clone(&self.block_device));
        let mut disk_inode_lock = cache.lock();
        let disk_inode = disk_inode_lock.as_mut_ref::<DiskInode>(self.block_offset);

        assert!(disk_inode.is_dir());
        let mut dir_entry = DirEntry::empty();
        let file_count = (disk_inode.size as usize) / DIR_ENTRY_SIZE;
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(
                    DIR_ENTRY_SIZE * i,
                    dir_entry.as_bytes_mut(),
                    &self.block_device
                ),
                DIR_ENTRY_SIZE,
            );
            if dir_entry.name() == name {
                let fs = self.fs.lock();
                let (block_id, block_offset) = fs.get_disk_inode_pos(dir_entry.inode_id());
                drop(fs);
                let inode = Inode::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                );
                if create {
                    inode.clear()
                }
                return Some(Arc::new(inode));
            }
        }
        drop(disk_inode_lock);
        drop(cache);

        // no such file in current directory
        if create {
            // create a new file
            let mut fs = self.fs.lock();

            // println!("access_dir_entry. got fs lock");

            let new_inode_id = fs.alloc_inode_block();
            let (new_block_id, new_block_offset) = fs.get_disk_inode_pos(new_inode_id);

            // println!("new_block_id: {}, new_block_offset: {}", new_block_id, new_block_offset);

            let cache = get_block_cache(new_block_id as usize, Arc::clone(&self.block_device));
            let mut new_disk_inode_lock = cache.lock();
            let new_disk_inode = new_disk_inode_lock.as_mut_ref::<DiskInode>(new_block_offset);

            // println!("access_dir_entry. got new cache");

            new_disk_inode.init(type_);

            drop(new_disk_inode_lock);
            drop(cache);

            // modify the current disk inode
            let cache = get_block_cache(self.block_id, Arc::clone(&self.block_device));
            let mut disk_inode_lock = cache.lock();
            let disk_inode = disk_inode_lock.as_mut_ref::<DiskInode>(self.block_offset);

            let file_count = (disk_inode.size as usize) / DIR_ENTRY_SIZE;
            let new_size = (file_count + 1) * DIR_ENTRY_SIZE;

            self.increase_size(new_size as u32, disk_inode,  &mut fs);

            let new_dir_entry = DirEntry::new(name, new_inode_id);
            disk_inode.write_at(
                file_count * DIR_ENTRY_SIZE,
                new_dir_entry.as_bytes(),
                &self.block_device,
            );

            drop(disk_inode_lock);
            drop(cache);

            sync_all();
            Some(Arc::new(Self::new(
                new_block_id,
                new_block_offset,
                self.fs.clone(),
                self.block_device.clone(),
            )))
        } else {
            None
        }
    }

    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();

        let cache = get_block_cache(self.block_id, Arc::clone(&self.block_device));
        let disk_inode_lock = cache.lock();
        let disk_inode = disk_inode_lock.as_ref::<DiskInode>(self.block_offset);

        disk_inode.read_at(offset, buf, &self.block_device)
    }
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();

        let cache = get_block_cache(self.block_id, Arc::clone(&self.block_device));
        let mut disk_inode_lock = cache.lock();
        let disk_inode = disk_inode_lock.as_mut_ref::<DiskInode>(self.block_offset);

        self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);

        // println!("write_at. increase_size ok");

        let write_size = disk_inode.write_at(offset, buf, &self.block_device);

        // println!("write_at. write ok");

        drop(disk_inode_lock);
        drop(cache);

        sync_all();
        write_size
    }

    /// List all inodes under current inode
    pub fn ls(&self) -> Vec<String> {
        let _fs = self.fs.lock();

        let cache = get_block_cache(self.block_id, Arc::clone(&self.block_device));
        let disk_inode_lock = cache.lock();
        let disk_inode = disk_inode_lock.as_ref::<DiskInode>(self.block_offset);

        let file_count = (disk_inode.size as usize) / DIR_ENTRY_SIZE;
        let mut v: Vec<String> = Vec::new();
        for i in 0..file_count {
            let mut dentry = DirEntry::empty();
            assert_eq!(
                disk_inode.read_at(
                    i * DIR_ENTRY_SIZE,
                    dentry.as_bytes_mut(),
                    &self.block_device
                ),
                DIR_ENTRY_SIZE,
            );
            v.push(String::from(dentry.name()));
        }
        v
    }
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        let cache = get_block_cache(self.block_id, Arc::clone(&self.block_device));
        let mut disk_inode_lock = cache.lock();
        let disk_inode = disk_inode_lock.as_mut_ref::<DiskInode>(self.block_offset);

        let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
        for block_id in data_blocks_dealloc {
            fs.dealloc_data_block(block_id);
        }
        sync_all()
    }
}
