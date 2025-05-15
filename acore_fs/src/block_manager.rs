use crate::block_cache::BlockCache;
use crate::block_dev::BlockDevice;
use alloc::sync::Arc;
use core::num::NonZeroUsize;
use lazy_static::lazy_static;
use lru::LruCache;
use spin::Mutex;

pub const BLOCK_CACHE_CAP: usize = 16;
pub struct BlockManager {
    lru_cache: LruCache<usize, Arc<Mutex<BlockCache>>>,
}

impl BlockManager {
    pub fn new() -> Self {
        BlockManager {
            lru_cache: LruCache::new(NonZeroUsize::try_from(BLOCK_CACHE_CAP).unwrap()),
        }
    }
    pub fn get_block_cache(
        &mut self,
        block_id: usize,
        block_device: Arc<dyn BlockDevice>,
    ) -> Arc<Mutex<BlockCache>> {
        if let Some(cache) = self.lru_cache.get(&block_id) {
            cache.clone()
        } else {
            if self.lru_cache.len() == BLOCK_CACHE_CAP {
                // evict
                let id = self.lru_cache.iter().rev().find_map(|(id, cache)| {
                    if Arc::strong_count(cache) == 1 {
                        Some(*id)
                    } else {
                        None
                    }
                });
                if let Some(id) = id {
                    self.lru_cache.pop(&id);
                } else {
                    panic!("Run out of BlockCache!");
                }
            }
            let cache = Arc::new(Mutex::new(BlockCache::new(
                block_id,
                Arc::clone(&block_device),
            )));
            self.lru_cache.put(block_id, Arc::clone(&cache));
            cache
        }
    }
}

lazy_static! {
    pub static ref BLOCK_MANAGER: Mutex<BlockManager> = Mutex::new(BlockManager::new());
}

pub fn get_block_cache(
    block_id: usize,
    block_device: Arc<dyn BlockDevice>,
) -> Arc<Mutex<BlockCache>> {
    BLOCK_MANAGER.lock().get_block_cache(block_id, block_device)
}

pub fn sync_all () {
    let mut block_manager = BLOCK_MANAGER.lock();
    for (_, cache) in block_manager.lru_cache.iter_mut() {
        cache.lock().sync();
    }
}
