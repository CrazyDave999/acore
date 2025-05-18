mod heap;
mod addr;
mod page_table;
mod frame_allocator;
mod mem_manager;
mod loader;

pub use mem_manager::{MemoryManager, KERNEL_MM, init_kernel_stack, release_kernel_stack,
                      get_kernel_stack_info};
pub use addr::{VirtAddr, PhysAddr, VirtPageNum, PhysPageNum};
pub use loader::{get_app_data_by_name, list_apps};
pub use page_table::PageTable;
pub use frame_allocator::{frame_alloc, FrameGuard, frame_dealloc};

pub fn init() {
    heap::init();
    frame_allocator::init();
    // KERNEL_MM.exclusive_access().page_table.visualize();
    KERNEL_MM.exclusive_access().activate();
}