use crate::bitmap::BitmapManager;
use crate::block_manager::{get_block_cache, sync_all};
use crate::layout::{DataBlock, DiskInode, DiskInodeType, SuperBlock, DISK_INODE_PER_BLOCK, DISK_INODE_SIZE};
use crate::{BlockDevice, Inode, BLOCK_SIZE};
use alloc::sync::Arc;
use spin::Mutex;
const BITS_PER_BLOCK: usize = BLOCK_SIZE * 8;

/// superblock, inode bitmap, data bitmap, inode, data
pub struct AcoreFileSystem {
    pub block_device: Arc<dyn BlockDevice>,
    pub inode_bitmap: BitmapManager,
    pub data_bitmap: BitmapManager,
    pub inode_start_block: u32,
    pub data_start_block: u32,
}

impl AcoreFileSystem {
    /// Create a new Acore file system with given parameters
    /// block_device: block device
    /// total_blocks: total blocks of the block device
    /// inode_num: max number of inodes
    pub fn new(
        block_device: Arc<dyn BlockDevice>,
        total_blocks: u32,
        inode_num: u32,
    ) -> Arc<Mutex<Self>> {
        let inode_blocks =
            (inode_num * core::mem::size_of::<DiskInode>() as u32 + BLOCK_SIZE as u32 - 1)
                / BLOCK_SIZE as u32;
        let inode_bitmap_blocks = (inode_num + BITS_PER_BLOCK as u32 - 1) / BITS_PER_BLOCK as u32;
        let data_total_blocks = total_blocks - 1 - inode_blocks - inode_bitmap_blocks;
        let data_bitmap_blocks = (data_total_blocks + BITS_PER_BLOCK as u32) / (BITS_PER_BLOCK as
            u32 + 1);
        let data_blocks = data_bitmap_blocks * BITS_PER_BLOCK as u32;

        let inode_bitmap = BitmapManager::new(1, inode_bitmap_blocks as usize);
        let data_bitmap = BitmapManager::new(
            (1 + inode_bitmap_blocks) as usize,
            data_bitmap_blocks as usize,
        );
        let inode_start_block =
            1 + inode_bitmap_blocks + data_bitmap_blocks;
        let data_start_block = inode_start_block + inode_blocks;
        let mut afs = Self {
            block_device: Arc::clone(&block_device),
            inode_bitmap,
            data_bitmap,
            inode_start_block,
            data_start_block,
        };
        // clear all blocks
        for i in 0..total_blocks {
            let cache = get_block_cache(i as usize, Arc::clone(&block_device));
            let mut data_block_lock = cache.lock();
            let data_block = data_block_lock.as_mut_ref::<DataBlock>(0);

            data_block.clear();
        }

        // initialize SuperBlock
        let cache = get_block_cache(0, Arc::clone(&block_device));
        let mut super_block_lock = cache.lock();
        let super_block = super_block_lock.as_mut_ref::<SuperBlock>(0);
        super_block.init(
            inode_bitmap_blocks,
            inode_blocks,
            data_bitmap_blocks,
            data_blocks,
        );

        // create root inode
        assert_eq!(afs.alloc_inode_block(), 0);
        let (root_block_id, root_block_offset) = afs.get_disk_inode_pos(0);

        let cache = get_block_cache(root_block_id as usize, Arc::clone(&block_device));
        let mut root_disk_inode_lock = cache.lock();
        let root_disk_inode = root_disk_inode_lock.as_mut_ref::<DiskInode>(root_block_offset);

        root_disk_inode.init(DiskInodeType::Directory);
        sync_all();
        Arc::new(Mutex::new(afs))
    }
    pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<Mutex<Self>> {
        let cache =  get_block_cache(0, Arc::clone(&block_device));
        let super_block_lock = cache.lock();
        let super_block = super_block_lock.as_ref::<SuperBlock>(0);

        assert!(super_block.is_valid(), "Invalid Acore File System");
        let inode_bitmap = BitmapManager::new(1, super_block.inode_bitmap_blocks as usize);
        let data_bitmap = BitmapManager::new(
            (1 + super_block.inode_bitmap_blocks) as usize,
            super_block.data_bitmap_blocks as usize,
        );
        let inode_start_block =
            1 + super_block.inode_bitmap_blocks + super_block.data_bitmap_blocks;
        let data_start_block = inode_start_block + super_block.inode_blocks;
        Arc::new(Mutex::new(Self {
            block_device: Arc::clone(&block_device),
            inode_bitmap,
            data_bitmap,
            inode_start_block,
            data_start_block,
        }))
    }
    pub fn alloc_inode_block(&mut self) -> u32 {
        self.inode_bitmap.alloc(&self.block_device).unwrap() as u32
    }
    pub fn alloc_data_block(&mut self) -> u32 {
        self.data_bitmap.alloc(&self.block_device).unwrap() as u32 + self.data_start_block
    }
    pub fn dealloc_data_block(&mut self, block_id: u32) {
        let cache = get_block_cache(block_id as usize, Arc::clone(&self.block_device));
        let mut data_block_lock = cache.lock();
        let data_block = data_block_lock.as_mut_ref::<DataBlock>(0);
        data_block.clear();
        self.data_bitmap.dealloc(
            &self.block_device,
            (block_id - self.data_start_block) as usize,
        );
    }

    /// Get the block id and offset of the inode
    pub fn get_disk_inode_pos(&self, inode_id: u32) -> (u32, usize) {
        (
            self.inode_start_block + inode_id / DISK_INODE_PER_BLOCK as u32,
            (inode_id as usize % DISK_INODE_PER_BLOCK) * DISK_INODE_SIZE
        )
    }

    pub fn root_inode(afs: Arc<Mutex<Self>>) -> Arc<Inode> {
        let (block_id, block_offset) = afs.lock().get_disk_inode_pos(0);
        Arc::new(Inode::new(
            block_id,
            block_offset,
            Arc::clone(&afs),
            Arc::clone(&afs.lock().block_device),
        ))
    }
}
