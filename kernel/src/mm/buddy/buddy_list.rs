/// a simple linked_list-like datastructure implementation
/// this is intrusive linked list, so the mem space actually has two forms:
/// 1. when it's free, it's a linked list node
/// 2. when it's allocated, it's the memory block



use core::cmp::min;
use core::ptr::null_mut;


#[derive(Copy, Clone)]
pub struct BuddyList {
    head: *mut usize,
    class: usize,
}

unsafe impl Send for BuddyList {}

impl BuddyList {
    pub const fn new() -> Self {
        Self {
            head: null_mut(),
            class: 0,
        }
    }
    pub fn init(&mut self, class: usize) {
        self.class = class;
    }

    pub fn is_empty(&self) -> bool {
        self.head.is_null()
    }

    /// push an item to the front of the list
    pub fn push(&mut self, item: *mut usize) {
        unsafe {
            *item = self.head as usize;
        }
        self.head = item;
    }

    pub fn pop(&mut self) -> Option<*mut usize> {
        if self.is_empty() {
            None
        } else {
            let item = self.head;
            self.head = unsafe { *item as *mut usize };
            Some(item)
        }
    }



    pub fn buddy(&self, x: usize) -> usize {
        x ^ (1 << self.class)
    }
    fn is_buddy(&self, x: *mut usize, y: *mut usize) -> Option<*mut usize> {
        if x as usize == self.buddy(y as usize) {
            Some(min(x, y))
        } else {
            None
        }
    }

    /// insert an item to the list, and merge it with the adjacent items if possible
    /// if merge happens, the merged term will be returned
    pub fn insert(&mut self, item: *mut usize) -> Option<*mut usize> {
        if self.class == 31 {
            // no need to merge
            self.push(item);
            return None;
        }
        let mut cur = self.head;
        if cur.is_null() {
            self.push(item);
            return None;
        }
        let mut prev = cur;
        unsafe { cur = *cur as *mut usize; }
        if cur.is_null() {
            return match self.is_buddy(prev, item) {
                Some(mi) => {
                    self.head = null_mut();
                    Some(mi)
                }
                _ => unsafe {
                    if prev < item {
                        *prev = item as usize;
                        *item = null_mut::<usize>() as usize;
                    } else {
                        *item = prev as usize;
                        self.head = item;
                    }
                    None
                }
            }
        }
        while !cur.is_null() && cur < item {
            match self.is_buddy(cur, item) {
                Some(mi) => {
                    unsafe { *prev = *cur; }
                    return Some(mi);
                }
                _ => {
                    prev = cur;
                    unsafe { cur = *cur as *mut usize; }
                }
            }
        }
        if cur.is_null() {
            unsafe {
                *prev = item as usize;
                *item = null_mut::<usize>() as usize;
                None
            }
        } else {
            // now cur > item
            match self.is_buddy(cur,item) {
                Some(mi) => {
                    unsafe { *prev = *cur; }
                    Some(mi)
                }
                _ => {
                    unsafe {
                        *item = cur as usize;
                        *prev = item as usize;
                    }
                    None
                }
            }
        }
    }
}
