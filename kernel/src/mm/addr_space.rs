use crate::mm::addr::VirtPageNum;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::arch::asm;
use riscv::register::satp;
use crate::mm::frame_allocator::FrameGuard;
use super::page_table::PageTable;

/// The manager of kernel space or user space
pub struct AddrSpace {
    /// responsible for modifying and reading page table's memory area
    page_table: PageTable,
    ///
    segments: Vec<Segment>,
    /// hold all frame guards during AddrSpace's lifetime
    frame_guards: BTreeMap<VirtPageNum, FrameGuard>,
}

/// A continuous memory region, with same flags
pub struct Segment {
    start_vpn: VirtPageNum,
    end_vpn: VirtPageNum,
}
impl AddrSpace {
    pub fn activate(&self) {
        let satp = self.page_table.token();
        unsafe {
            satp::write(satp);
            asm!("sfence.vma");
        }
    }
}