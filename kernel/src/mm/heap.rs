use crate::config::KERNEL_HEAP_SIZE;
use buddy::Heap;

#[global_allocator]
static KERNEL_HEAP: Heap = Heap::new();

#[alloc_error_handler]
pub fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error: {:?}", layout)
}

static mut KERNEL_HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

pub fn init() {
    unsafe {
        KERNEL_HEAP.borrow_mut().init(KERNEL_HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE);
    }
}