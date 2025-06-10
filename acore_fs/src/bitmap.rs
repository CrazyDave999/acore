use alloc::sync::Arc;
use crate::block_dev::BlockDevice;
use crate::block_manager::get_block_cache;
use crate::BLOCK_SIZE;

pub struct BitmapManager {
    start_block_id: usize,
    len: usize,
}
type BitmapBlock = [u64; BLOCK_SIZE / core::mem::size_of::<u64>()]; // 512 bytes

impl BitmapManager {
    pub fn new(start_block_id: usize, len: usize) -> Self {
        Self {
            start_block_id,
            len,
        }
    }
    pub fn alloc(&self, block_device: &Arc<dyn BlockDevice>) -> Option<usize> {
        for block_pos in 0..self.len {
            let cache = get_block_cache(
                self.start_block_id + block_pos,
                Arc::clone(block_device),
            );
            let mut bitmap_block_lock = cache.lock();
            let bitmap_block = bitmap_block_lock.as_mut_ref::<BitmapBlock>(0);

            for (bits64_pos, bits64) in bitmap_block.iter_mut().enumerate() {
                if *bits64 != u64::MAX {
                    let inner_pos = bits64.trailing_ones() as usize;
                    *bits64 |= 1u64 << inner_pos;
                    return Some(
                        block_pos * BLOCK_BITS + bits64_pos * 64 + inner_pos,
                    )
                }
            }
        }
        None
    }
    pub fn dealloc(&self, block_device: &Arc<dyn BlockDevice>, id: usize) {
        let block_pos = id / BLOCK_BITS;
        let bits64_pos = (id % BLOCK_BITS) / 64;
        let inner_pos = (id % BLOCK_BITS) % 64;
        let cache = get_block_cache(
            self.start_block_id + block_pos,
            Arc::clone(block_device),
        );
        let mut bitmap_block_lock = cache.lock();
        let bitmap_block = bitmap_block_lock.as_mut_ref::<BitmapBlock>(0);
        assert_ne!(bitmap_block[bits64_pos] & (1u64 << inner_pos), 0);
        bitmap_block[bits64_pos] ^= 1u64 << inner_pos;
    }
}

const BLOCK_BITS: usize = BLOCK_SIZE * 8;


