use super::page_table::{PTEFlags, PageTable};
use crate::config::*;
use crate::mm::addr::{VirtAddr, VirtPageNum};
use crate::mm::frame_allocator::{frame_alloc, FrameGuard};
use crate::mm::PhysPageNum;
use crate::println;
use crate::sync::UPSafeCell;
use crate::utils::NumRange;
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
            start_vpn: start_va.floor(),
            end_vpn: end_va.ceil(),
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
        println!("for ph start");
        for i in 0..ph_cnt {
            let ph = elf.program_header(i).unwrap();
            match ph.get_type() {
                Ok(ty) => {
                    if ty == Type::Load {
                        let start_va = VirtAddr(ph.virtual_addr() as usize);
                        let end_va = VirtAddr((ph.virtual_addr() + ph.mem_size()) as usize);
                        println!(
                            "i = {}, start_vpn = {:?}, end_vpn = {:?}, start_va = {:?}, \
                        end_va = {:?}",
                            i,
                            start_va.floor(),
                            end_va.ceil(),
                            start_va,
                            end_va
                        );
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
        let user_stack_bottom: usize = max_end_va.ceil().0 + PAGE_SIZE;
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

    /// Clone a memory manager. For fork.
    pub fn from_existed(another_mm: &Self) -> Self {
        let mut mm = MemoryManager::empty();
        mm.entry_point = another_mm.entry_point;
        mm.user_stack_top = another_mm.user_stack_top;
        mm.map_trampoline();
        for area in another_mm.areas.iter() {
            mm.push_area(
                area.start_vpn.into(),
                area.end_vpn.into(),
                area.map_type,
                area.map_perm,
                None,
            );
            for vpn in NumRange::new(area.start_vpn, area.end_vpn) {
                let src_data = another_mm
                    .page_table
                    .find_ppn(vpn)
                    .unwrap()
                    .get_bytes_array();
                let dst_data = mm.page_table.find_ppn(vpn).unwrap().get_bytes_array();
                dst_data.copy_from_slice(src_data);
            }
        }
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
        for vpn in NumRange::<VirtPageNum>::new(start_va.floor(), end_va.ceil()) {
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
    pub fn push_area(
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
        let mut cur_dst_vpn = start_va.floor();
        let mut cur_src_start: usize = 0;
        let mut cur_dst_start = start_va.get_page_offset();
        let end: usize = data.len();
        loop {
            let cur_src_end = min(cur_src_start + PAGE_SIZE, end);
            let src = &data[cur_src_start..cur_src_end];
            let dst = &mut self
                .page_table
                .find_pte(cur_dst_vpn)
                .unwrap()
                .ppn()
                .get_bytes_array()[cur_dst_start..cur_dst_start + src.len()];
            dst.copy_from_slice(src);
            cur_src_start += PAGE_SIZE;
            if cur_src_start >= end {
                break;
            }
            cur_dst_start = 0;
            cur_dst_vpn.0 += 1;
        }
    }
    pub fn recycle_data_pages(&mut self) {
        self.areas.clear();
    }
}

impl Area {}

lazy_static! {
    pub static ref KERNEL_MM: Arc<UPSafeCell<MemoryManager>> =
        Arc::new(unsafe { UPSafeCell::new(MemoryManager::new_kernel()) });
}

/// Get the kernel stack position of given pid
pub fn get_kernel_stack_info(pid: usize) -> (usize, usize) {
    let top = TRAMPOLINE - pid * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}

/// Create framed kernel stack space for given pid
pub fn init_kernel_stack(pid: usize) -> (usize, usize) {
    let (bottom, top) = get_kernel_stack_info(pid);
    KERNEL_MM.exclusive_access().push_area(
        bottom.into(),
        top.into(),
        MapType::Framed,
        MapPerm::R | MapPerm::W,
        None,
    );
    (bottom, top)
}
