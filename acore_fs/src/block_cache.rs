use alloc::sync::Arc;
use crate::block_dev::BlockDevice;
use crate::BLOCK_SIZE;

pub struct BlockCache {
    data: [u8; BLOCK_SIZE],
    block_id: usize,
    block_device: Arc<dyn BlockDevice>,
    dirty: bool,
}

impl BlockCache {
    pub fn new(block_id: usize, block_device: Arc<dyn BlockDevice>) -> Self {
        let mut data = [0u8; BLOCK_SIZE];
        block_device.read_block(block_id, &mut data);
        Self {
            data,
            block_id,
            block_device,
            dirty: false,
        }
    }
    pub fn as_ref<T>(&self, offset: usize) -> &T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SIZE);
        unsafe { &*(self.data.as_ptr().add(offset) as *const T) }
    }
    pub fn as_mut_ref<T>(&mut self, offset: usize) -> &mut T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SIZE);
        self.dirty = true;
        unsafe { &mut *(self.data.as_mut_ptr().add(offset) as *mut T) }
    }



    pub fn sync(&mut self) {
        if self.dirty {
            self.dirty = false;
            self.block_device.write_block(self.block_id, &self.data);
        }
    }

}
impl Drop for BlockCache {
    fn drop(&mut self) {
        self.sync();
    }
}