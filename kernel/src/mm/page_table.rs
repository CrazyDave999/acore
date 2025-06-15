use super::addr::{PhysPageNum, VirtPageNum};
use crate::config::*;
use crate::mm::frame_allocator::{frame_alloc, FrameGuard};
use crate::mm::{PhysAddr, VirtAddr};
use crate::println;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use bitflags::*;
use core::ops::AddAssign;

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
        PageTableEntry { bits: 0 }
    }
    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << PPN_WIDTH_SV39) - 1)).into()
    }
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }
    pub fn is_user(&self) -> bool {
        (self.flags() & PTEFlags::U) != PTEFlags::empty()
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

#[derive(Debug)]
pub struct PageTable {
    root_ppn: PhysPageNum,
    /// The frame guards of all frames used by this page table, not used by actual data contents
    frame_guards: Vec<FrameGuard>,
}

impl AddAssign<i32> for VirtPageNum {
    fn add_assign(&mut self, rhs: i32) {
        let new_vpn = self.0 as i32 + rhs;
        if new_vpn < 0 {
            panic!("VPN underflow! vpn: {:?}", self);
        }
        self.0 = new_vpn as usize;
    }
}

impl PageTable {
    pub fn empty() -> Self {
        let frame = frame_alloc().unwrap();
        PageTable {
            root_ppn: frame.ppn,
            frame_guards: vec![frame],
        }
    }
    /// Create a temporary page table, used for fetching user space data in kernel mode
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: (satp & ((1usize << 44) - 1)).into(),
            frame_guards: vec![],
        }
    }
    /// Create a virtual-physical mapping
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        // println!("token: {:#x}, map: vpn: {:#x}, ppn: {:#x}", self.root_ppn.0, vpn.0, ppn.0);
        let pte = self.find_mut_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "Mapping existed! vpn: {:?}", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }

    /// Remove a virtual-physical mapping
    #[allow(unused)]
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_mut_pte(vpn).unwrap();
        assert!(
            pte.is_valid(),
            "Trying to unmap a non-existed mapping! vpn: {:?}",
            vpn
        );
        *pte = PageTableEntry::empty();
    }

    /// The identifier of the page table
    pub fn token(&self) -> usize {
        // SV39 mode is 0b1000
        self.root_ppn.0 | (1usize << 63)
    }

    fn find_mut_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, &ind) in vpn.get_indexes().iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[ind];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        result
    }
    pub fn find_pte(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_mut_pte(vpn).map(|pte| pte.clone())
    }
    pub fn find_ppn(&self, vpn: VirtPageNum) -> Option<PhysPageNum> {
        self.find_pte(vpn).map(|pte| pte.ppn())
    }
    pub fn find_pa(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.find_pte(va.clone().floor()).map(|pte|{
            let aligned_pa: PhysAddr = pte.ppn().into();
            let offset = va.get_page_offset();
            (aligned_pa.0 + offset).into()
        })
    }
    pub fn find_mut_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, &ind) in vpn.get_indexes().iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[ind];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frame_guards.push(frame);
            }
            ppn = pte.ppn();
        }
        result
    }
    // /// find a str with start va, terminate when meet \0, but not include \0
    // pub fn find_str(&self, va: VirtAddr) -> String {
    //     let mut s = String::new();
    //     let mut offset = va.get_page_offset();
    //     let mut cur_vpn = va.floor();
    //     loop {
    //         let data = self.find_ppn(cur_vpn).unwrap().get_bytes_array();
    //         let mut terminated = false;
    //         while offset < PAGE_SIZE {
    //             if data[offset] == 0 {
    //                 terminated = true;
    //                 break;
    //             }
    //             s.push(data[offset] as char);
    //             offset += 1;
    //         }
    //         if terminated {
    //             break;
    //         }
    //         offset = 0;
    //         cur_vpn += 1;
    //     }
    //     s
    // }
    #[allow(unused)]
    pub fn visualize(&self) {
        Self::visualize_dfs(self.root_ppn, 0);
    }

    #[allow(unused)]
    pub fn visualize_dfs(ppn: PhysPageNum, dep: usize) {
        println!("{}Page Table: {:#x}", "  ".repeat(dep), ppn.0);
        for i in 0..512 {
            let pte = &ppn.get_pte_array()[i];

            if !pte.is_valid() {
                continue;
            }
            println!(
                "{}Entry {}, ppn: {:#x}, U: {}, X: {}, W: {}, R: {}, V: {}",
                "  ".repeat(dep),
                i,
                pte.ppn().0,
                pte.is_user(),
                pte.is_executable(),
                pte.is_writable(),
                pte.is_readable(),
                pte.is_valid()
            );

            if pte.is_valid() && dep < 2 {
                Self::visualize_dfs(pte.ppn(), dep + 1);
            }
        }
    }
}
