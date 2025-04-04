use super::page_table::{PTEFlags, PageTable};
use crate::config::*;
use crate::mm::addr::{VirtAddr, VirtPageNum};
use crate::mm::frame_allocator::{frame_alloc, FrameGuard};
use crate::mm::PhysPageNum;
use crate::sync::UPSafeCell;
use alloc::sync::Arc;
use alloc::vec::Vec;
use bitflags::bitflags;
use core::arch::asm;
use core::cmp::{max, min};
use lazy_static::lazy_static;
use riscv::register::satp;
use xmas_elf::program::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapType {
    Identical,
    Framed,
}

bitflags! {
    pub struct MapPerm: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

extern "C" {
    fn stext();
    fn etext();
    fn srodata();
    fn erodata();
    fn sdata();
    fn edata();
    fn sbss_with_stack();
    fn ebss();
    fn ekernel();
    fn strampoline();
}

/// The manager of kernel space or user space
pub struct MemoryManager {
    /// responsible for modifying and reading page table's memory area
    pub page_table: PageTable,

    pub areas: Vec<Area>,

    pub entry_point: usize,

    pub user_stack_top: usize,
}

/// A continuous memory region, with same flags
pub struct Area {
    start_vpn: VirtPageNum,
    end_vpn: VirtPageNum,
    frame_guards: Vec<FrameGuard>,
    map_type: MapType,
    map_perm: MapPerm,
}

impl Area {
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPerm,
        frame_guards: Vec<FrameGuard>,
    ) -> Self {
        Self {
            start_vpn: start_va.into(),
            end_vpn: end_va.into(),
            map_type,
            map_perm,
            frame_guards,
        }
    }
}

impl MemoryManager {
    pub fn empty() -> Self {
        MemoryManager {
            page_table: PageTable::empty(),
            areas: Vec::new(),
            entry_point: 0,
            user_stack_top: 0,
        }
    }

    pub fn new_kernel() -> Self {
        let mut mm = MemoryManager::empty();
        mm.map_trampoline();
        mm.push_area(
            (stext as usize).into(),
            (etext as usize).into(),
            MapType::Identical,
            MapPerm::R | MapPerm::X,
            None,
        );
        mm.push_area(
            (srodata as usize).into(),
            (erodata as usize).into(),
            MapType::Identical,
            MapPerm::R,
            None,
        );
        mm.push_area(
            (sdata as usize).into(),
            (edata as usize).into(),
            MapType::Identical,
            MapPerm::R | MapPerm::W,
            None,
        );
        mm.push_area(
            (sbss_with_stack as usize).into(),
            (ebss as usize).into(),
            MapType::Identical,
            MapPerm::R | MapPerm::W,
            None,
        );
        mm.push_area(
            (ekernel as usize).into(),
            MEMORY_END.into(),
            MapType::Identical,
            MapPerm::R | MapPerm::W,
            None,
        );
        // MMIO region
        mm.push_area(
            VIRT_TEST.into(),
            (VIRT_TEST + 0x2000).into(), // VIRT_TEST and VIRT_RTC
            MapType::Identical,
            MapPerm::R | MapPerm::W,
            None,
        );
        mm
    }
    pub fn from_elf(data: &[u8]) -> Self {
        let mut mm = MemoryManager::empty();
        mm.map_trampoline();
        let elf = xmas_elf::ElfFile::new(data).unwrap();
        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "Invalid ELF magic number");
        let ph_cnt = elf_header.pt2.ph_count();
        let mut max_end_va = VirtAddr(0);
        for i in 0..ph_cnt {
            let ph = elf.program_header(i).unwrap();
            match ph.get_type() {
                Ok(ty) => {
                    if ty == Type::Load {
                        let start_va = (ph.virtual_addr() as usize).into();
                        let end_va = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
                        max_end_va = max(max_end_va, end_va);
                        let mut map_perm = MapPerm::U;
                        let ph_flags = ph.flags();
                        if ph_flags.is_read() {
                            map_perm |= MapPerm::R;
                        }
                        if ph_flags.is_write() {
                            map_perm |= MapPerm::W;
                        }
                        if ph_flags.is_execute() {
                            map_perm |= MapPerm::X;
                        }
                        mm.push_area(
                            start_va,
                            end_va,
                            MapType::Framed,
                            map_perm,
                            Some(
                                &elf.input
                                    [ph.offset() as usize..(ph.offset() + ph.file_size()) as usize],
                            ),
                        );
                    }
                }
                Err(_) => {
                    panic!("Invalid ELF program header type")
                }
            }
        }
        // align end_of_elf_data
        let user_stack_bottom: usize = max_end_va.ceil().into() + PAGE_SIZE;
        let user_stack_top: usize = user_stack_bottom + USER_STACK_SIZE;
        // user stack
        mm.push_area(
            user_stack_bottom.into(),
            user_stack_top.into(),
            MapType::Framed,
            MapPerm::R | MapPerm::W | MapPerm::U,
            None,
        );
        // trap context
        mm.push_area(
            TRAP_CONTEXT.into(),
            TRAMPOLINE.into(),
            MapType::Framed,
            MapPerm::R | MapPerm::W,
            None,
        );
        mm.user_stack_top = user_stack_top;
        mm.entry_point = elf.header.pt2.entry_point() as usize;
        mm
    }
    fn get_area_frame_guards(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPerm,
    ) -> Vec<FrameGuard> {
        let mut frame_guards = Vec::new();
        for vpn in start_va.into()..end_va.into() {
            let ppn: PhysPageNum;
            match map_type {
                MapType::Identical => {
                    ppn = PhysPageNum(vpn.0);
                }
                MapType::Framed => {
                    let frame = frame_alloc().unwrap();
                    ppn = frame.ppn;
                    frame_guards.push(frame);
                }
            }
            self.page_table
                .map(vpn, ppn, PTEFlags::from_bits(map_perm.bits).unwrap());
        }
        frame_guards
    }
    fn map_trampoline(&mut self) {
        self.page_table.map(
            TRAMPOLINE.into(),
            (strampoline as usize).into(),
            PTEFlags::R | PTEFlags::X,
        )
    }
    fn push_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPerm,
        data: Option<&[u8]>,
    ) {
        let frame_guards = self.get_area_frame_guards(start_va, end_va, map_type, map_perm);
        let area = Area::new(start_va, end_va, map_type, map_perm, frame_guards);
        self.areas.push(area);
        if let Some(data) = data {
            self.write(start_va, data);
        }
    }

    pub fn activate(&self) {
        let satp = self.page_table.token();
        unsafe {
            satp::write(satp);
            asm!("sfence.vma");
        }
    }
    /// copy data to the specified virtual address
    pub fn write(&self, start_va: VirtAddr, data: &[u8]) {
        // the operation is carried out page by page, hence need align
        let mut cur_vpn = start_va.floor();
        let mut cur_start: usize = 0;
        let end: usize = data.len();
        loop {
            let cur_end = min(cur_start + PAGE_SIZE, end);
            let src = &data[cur_start..cur_end];
            let dst = &mut self
                .page_table
                .find_pte(cur_vpn)
                .unwrap()
                .ppn()
                .get_bytes_array()[..src.len()];
            dst.copy_from_slice(src);
            cur_start += PAGE_SIZE;
            if cur_start >= end {
                break;
            }
            cur_vpn.0 += 1;
        }
    }
}

impl Area {}

lazy_static! {
    pub static ref KERNEL_MM: Arc<UPSafeCell<MemoryManager>> =
        Arc::new(unsafe { UPSafeCell::new(MemoryManager::new_kernel()) });
}
