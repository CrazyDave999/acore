mod heap;
mod addr;
mod page_table;
mod frame_allocator;
mod mem_manager;
mod loader;

pub use addr::{PhysAddr, PhysPageNum, VirtAddr};
pub use frame_allocator::{frame_alloc, frame_dealloc, FrameGuard};
pub use mem_manager::{MapPerm, MapType, MemoryManager, KERNEL_MM
};
// pub use loader::{get_app_data_by_name, list_apps};
pub use page_table::PageTable;

pub fn init() {
    heap::init();
    frame_allocator::init();
    // KERNEL_MM.exclusive_access().page_table.visualize();
    KERNEL_MM.exclusive_access().activate();
}