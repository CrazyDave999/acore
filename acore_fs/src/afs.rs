use crate::bitmap::BitmapManager;
use crate::block_manager::get_block_cache;
use crate::layout::DataBlock;
use crate::BlockDevice;
use alloc::sync::Arc;
use spin::Mutex;

/// superblock, inode bitmap, data bitmap, inode, data
pub struct AcoreFileSystem {
    pub block_device: Arc<dyn BlockDevice>,
    pub inode_bitmap: BitmapManager,
    pub data_bitmap: BitmapManager,
    pub inode_start_block: u32,
    pub data_start_block: u32,
}

impl AcoreFileSystem {
    pub fn new(block_device: Arc<dyn BlockDevice>, data_block_num: usize) -> Arc<Mutex<Self>> {
        todo!()
    }
    pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<Mutex<Self>> {
        let super_block = get_block_cache(0, Arc::clone(&block_device)).lock();
        todo!()
    }
    pub fn alloc_inode_block(&mut self) -> u32 {
        self.inode_bitmap.alloc(&self.block_device).unwrap() as u32
    }
    pub fn alloc_data_block(&mut self) -> u32 {
        self.data_bitmap.alloc(&self.block_device).unwrap() as u32 + self.data_start_block
    }
    pub fn dealloc_data_block(&mut self, block_id: u32) {
        let data_block = get_block_cache(block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .as_mut_ref::<DataBlock>(0);
        data_block.clear();
        self.data_bitmap.dealloc(
            &self.block_device,
            (block_id - self.data_start_block) as usize,
        );
    }
}
