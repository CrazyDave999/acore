use super::addr::{PhysAddr, PhysPageNum};
use crate::config::*;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use crate::sync::UPSafeCell;
use crate::println;

#[derive(Debug)]
/// RAII style manipulation of frame allocation
pub struct FrameGuard {
    pub ppn: PhysPageNum,
}

impl FrameGuard {
    pub fn new(ppn: PhysPageNum) -> Self {
        ppn.clear_page();
        Self { ppn }
    }
}

impl Drop for FrameGuard {
    fn drop(&mut self) {
        frame_dealloc(self.ppn);
    }
}

trait FrameAllocator {
    fn new() -> Self;
    fn alloc(&mut self) -> Option<PhysPageNum>;
    fn dealloc(&mut self, ppn: PhysPageNum);
}

pub struct StackFrameAllocator {
    cap: usize,
    max_cap: usize,
    recycled: Vec<usize>,
}

impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {
        Self {
            cap: 0,
            max_cap: 0,
            recycled: Vec::new(),
        }
    }
    fn alloc(&mut self) -> Option<PhysPageNum> {
        if let Some(ppn) = self.recycled.pop() {
            Some(ppn.into())
        } else if self.cap == self.max_cap {
            // todo: all frames are used, consider evicting some frames to disk
            None
        } else {
            let ppn = self.cap;
            self.cap += 1;
            Some(ppn.into())
        }
    }
    fn dealloc(&mut self, ppn: PhysPageNum) {
        let ppn = ppn.0;
        if ppn >= self.cap || self.recycled.iter().any(|&v| v == ppn) {
            panic!("Frame ppn={:#x} has not been allocated!", ppn);
        }
        self.recycled.push(ppn);
    }
}

impl StackFrameAllocator {
    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        self.cap = l.0;
        self.max_cap = r.0;
        println!("last {} Physical Frames.", self.max_cap - self.cap);
    }
}

lazy_static!{
    pub static ref FRAME_ALLOCATOR: UPSafeCell<StackFrameAllocator> = unsafe {UPSafeCell::new(StackFrameAllocator::new())};
}

pub fn init() {
    extern "C" {
        // end of kernel
        fn ekernel();
    }
    FRAME_ALLOCATOR.exclusive_access().init(
        PhysAddr::from(ekernel as usize).ceil(),
        PhysAddr::from(MEMORY_END).floor(),
    );
}

pub fn frame_alloc() -> Option<FrameGuard> {
    let ppn = FRAME_ALLOCATOR.exclusive_access().alloc()?;
    Some(FrameGuard::new(ppn))
}
pub fn frame_dealloc(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.exclusive_access().dealloc(ppn);
}

