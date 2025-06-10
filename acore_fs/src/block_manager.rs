use alloc::collections::VecDeque;
use crate::block_cache::BlockCache;
use crate::block_dev::BlockDevice;
use alloc::sync::Arc;

use lazy_static::lazy_static;

use spin::Mutex;

pub const BLOCK_CACHE_CAP: usize = 16;
pub struct BlockManager {
    queue: VecDeque<(usize, Arc<Mutex<BlockCache>>)>,
}

impl BlockManager {
    pub fn new() -> Self {
        BlockManager {
            queue: VecDeque::new(),
        }
    }
    /// Get a block cache from the block device. Load it from disk if not in cache.
    pub fn get_block_cache(
        &mut self,
        block_id: usize,
        block_device: Arc<dyn BlockDevice>,
    ) -> Arc<Mutex<BlockCache>> {
        if let Some((_, cache)) = self.queue.iter().find(|(id,_)| *id == block_id) {
            cache.clone()
        } else {
            if self.queue.len() == BLOCK_CACHE_CAP {
                // evict
                if let Some((idx, _)) = self
                    .queue
                    .iter()
                    .enumerate()
                    .find(|(_, (_, cache))| Arc::strong_count(cache) == 1)
                {
                    self.queue.drain(idx..=idx);
                } else {
                    panic!("Run out of BlockCache!");
                }
            }
            let cache = Arc::new(Mutex::new(BlockCache::new(
                block_id,
                Arc::clone(&block_device),
            )));
            self.queue.push_back((block_id, Arc::clone(&cache)));
            cache
        }
    }
}

lazy_static! {
    pub static ref BLOCK_MANAGER: Mutex<BlockManager> = Mutex::new(BlockManager::new());
}

/// Get a block cache from the block device. Load it from disk if not in cache.
pub fn get_block_cache(
    block_id: usize,
    block_device: Arc<dyn BlockDevice>,
) -> Arc<Mutex<BlockCache>> {
    BLOCK_MANAGER.lock().get_block_cache(block_id, block_device)
}

/// Sync all block caches to disk
pub fn sync_all () {
    let block_manager = BLOCK_MANAGER.lock();
    for (_, cache) in block_manager.queue.iter() {
        cache.lock().sync();
    }
}
