pub mod buddy;
mod heap;
mod addr;
mod page_table;
mod frame_allocator;
mod addr_space;

pub fn init() {
    heap::init();
}