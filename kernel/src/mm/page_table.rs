use bitflags::*;
use crate::config::*;
use crate::mm::frame_allocator::{frame_alloc, FrameGuard};
use super::addr::{PhysPageNum, VirtPageNum};
use alloc::vec::Vec;

bitflags! {
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}


#[derive(Copy, Clone)]
#[repr(C)]
pub struct PageTableEntry {
    pub bits: usize,
}

impl PageTableEntry {
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }
    pub fn empty() -> Self {
        PageTableEntry {
            bits: 0,
        }
    }
    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << PPN_WIDTH_SV39) - 1)).into()
    }
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }
    pub fn is_readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }

    pub fn is_writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }

    pub fn is_executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

pub struct PageTable {
    root_ppn: PhysPageNum,
    /// The frame guards of all frames used by this page table
    frame_guards: Vec<FrameGuard>,
}

impl PageTable {
    /// Create a virtual-physical mapping
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let mut cur_ppn = self.root_ppn;
        for (i, &ind) in vpn.get_indexes().iter().enumerate() {
            let pte = &mut cur_ppn.get_pte_array()[ind];
            if i == 2 {
                if pte.is_valid() {
                    panic!("[kernel] Trying to build an existed mapping! vpn: {:?}", vpn);
                }
                *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
                return;
            }
            if !pte.is_valid() {
                if let Some(frame) = frame_alloc() {
                    *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                    self.frame_guards.push(frame);
                } else {
                    panic!("[kernel] Crashed! Too many frames!");
                }
            }
            cur_ppn = pte.ppn();
        }
    }

    /// Remove a virtual-physical mapping
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let mut cur_ppn = self.root_ppn;
        for (i, &ind) in vpn.get_indexes().iter().enumerate() {
            let pte = &mut cur_ppn.get_pte_array()[ind];
            if i == 2 {
                if !pte.is_valid() {
                    panic!("[kernel] Trying to remove a non-existed mapping! vpn: {:?}", vpn);
                }
                *pte = PageTableEntry::empty();
                return;
            }
            if !pte.is_valid() {
                panic!("[kernel] Trying to remove a non-existed mapping! vpn: {:?}", vpn);
            }
            cur_ppn = pte.ppn();
        }
    }

    /// The identifier of the page table
    pub fn token(&self) -> usize {
        self.root_ppn.0
    }
}
