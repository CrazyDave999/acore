use super::buddy_list::BuddyList;
use core::alloc::{GlobalAlloc, Layout};
use core::cmp::{max, min};
use core::cell::RefCell;
use core::mem::size_of;
use core::ops::Deref;
// use crate::config::*;

pub struct HeapInner {
    free_lists: [BuddyList; 32],
}

impl HeapInner {
    pub const fn new() -> Self {
        HeapInner {
            free_lists: [BuddyList::new(); 32],
        }
    }

    pub unsafe fn init(&mut self, start: usize, size: usize) {
        for i in 0..32 {
            self.free_lists[i].init(i);
        }

        // start shifted to the right and end shifted to the left, aligned to the size of usize
        let aligned_start = (start + size_of::<usize>() - 1) & (!size_of::<usize>() + 1);
        let aligned_end = (start + size) & (!size_of::<usize>() + 1);
        assert!(start <= aligned_end);

        let mut cur_start = aligned_start;
        while cur_start + size_of::<usize>() <= aligned_end {
            let cur_size = min(
                lowbit(cur_start),
                prev_power_of_two(aligned_end - cur_start),
            );
            self.free_lists[cur_size.trailing_zeros() as usize].push(cur_start as *mut usize);
            cur_start += cur_size;
        }
    }

    pub fn alloc(&mut self, layout: Layout) -> Option<*mut u8> {
        let size = max(
            layout.size().next_power_of_two(),
            max(layout.align(), size_of::<usize>()),
        );
        let class = size.trailing_zeros() as usize;
        for i in class..self.free_lists.len() {
            if let Some(ptr) = self.free_lists[i].pop() {
                // found block big enough, now check whether we need to split
                // from i - 1 to class
                for j in (class..i).rev() {
                    let buddy_of_ptr = self.free_lists[j].buddy(ptr as usize);
                    self.free_lists[j].insert(buddy_of_ptr as *mut usize);
                }
                return Some(ptr as *mut u8);
            }
        }
        panic!("[alloc] Out of memory");
    }

    pub fn dealloc(&mut self, mut ptr: *mut u8, layout: Layout) {
        let size = max(
            layout.size().next_power_of_two(),
            max(layout.align(), size_of::<usize>()),
        );
        let class = size.trailing_zeros() as usize;
        for i in class..self.free_lists.len() {
            if let Some(merged) = self.free_lists[i].insert(ptr as *mut usize) {
                ptr = merged as *mut u8;
            } else {
                break;
            }
        }
    }
}

fn lowbit(x: usize) -> usize {
    x & (!x + 1)
}

fn prev_power_of_two(num: usize) -> usize {
    1 << (8 * (size_of::<usize>()) - num.leading_zeros() as usize - 1)
}

pub struct Heap(RefCell<HeapInner>);

impl Heap {
    pub const fn new() -> Self {
        Self(RefCell::new(HeapInner::new()))
    }
}

impl Deref for Heap {
    type Target = RefCell<HeapInner>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

unsafe impl Sync for Heap {}

unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.0.borrow_mut().alloc(layout).unwrap_or(0 as *mut u8)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0.borrow_mut().dealloc(ptr, layout);
    }
}