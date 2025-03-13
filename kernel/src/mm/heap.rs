use crate::config::KERNEL_HEAP_SIZE;
use super::buddy::Heap;

#[global_allocator]
static HEAP: Heap = Heap::new();

#[alloc_error_handler]
pub fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error: {:?}", layout)
}

static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

pub fn init() {
    unsafe {
        HEAP.borrow_mut().init(HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE);
    }
}