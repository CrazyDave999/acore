pub mod buddy;
mod heap;
mod addr;
mod page_table;
mod frame_allocator;
mod mem_manager;

pub use addr::PhysPageNum;
pub use mem_manager::MemoryManager;

pub fn init() {
    heap::init();
}