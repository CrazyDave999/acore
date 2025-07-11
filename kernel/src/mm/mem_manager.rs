use super::page_table::{PTEFlags, PageTable};
use crate::config::*;
use crate::mm::addr::{PhysAddr, VirtAddr, VirtPageNum};
use crate::mm::frame_allocator::{frame_alloc, FrameGuard};
use crate::mm::PhysPageNum;
use crate::sync::UPSafeCell;
use crate::utils::NumRange;
use alloc::collections::BTreeMap;
use alloc::string::String;
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

    pub areas: BTreeMap<usize, Area>,

    pub entry_point: usize,

    pub user_stack_bottom: usize,
}

/// A continuous memory region, with same flags
pub struct Area {
    start_vpn: VirtPageNum,
    end_vpn: VirtPageNum,
    #[allow(unused)]
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
            areas: BTreeMap::new(),
            entry_point: 0,
            user_stack_bottom: 0,
        }
    }

    pub fn new_kernel() -> Self {
        let mut mm = MemoryManager::empty();
        mm.map_trampoline();
        // println!(".text [{:#x}, {:#x})", stext as usize, etext as usize);
        // println!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
        // println!(".data [{:#x}, {:#x})", sdata as usize, edata as usize);
        // println!(
        //     ".bss [{:#x}, {:#x})",
        //     sbss_with_stack as usize, ebss as usize
        // );

        mm.insert_area(
            (stext as usize).into(),
            (etext as usize).into(),
            MapType::Identical,
            MapPerm::R | MapPerm::X,
            None,
        );
        mm.insert_area(
            (srodata as usize).into(),
            (erodata as usize).into(),
            MapType::Identical,
            MapPerm::R,
            None,
        );
        mm.insert_area(
            (sdata as usize).into(),
            (edata as usize).into(),
            MapType::Identical,
            MapPerm::R | MapPerm::W,
            None,
        );
        mm.insert_area(
            (sbss_with_stack as usize).into(),
            (ebss as usize).into(),
            MapType::Identical,
            MapPerm::R | MapPerm::W,
            None,
        );
        mm.insert_area(
            (ekernel as usize).into(),
            MEMORY_END.into(),
            MapType::Identical,
            MapPerm::R | MapPerm::W,
            None,
        );

        // VIRT_TEST and VIRT_RTC, for shutdown
        mm.insert_area(
            VIRT_TEST.into(),
            (VIRT_TEST + 0x2000).into(),
            MapType::Identical,
            MapPerm::R | MapPerm::W,
            None,
        );

        // VIRT_CLINT, for timer
        mm.insert_area(
            VIRT_CLINT.into(),
            (VIRT_CLINT + VIRT_CLINT_SIZE).into(),
            MapType::Identical,
            MapPerm::R | MapPerm::W,
            None,
        );

        // VIRT_UART0, for uart
        mm.insert_area(
            VIRT_UART0.into(),
            (VIRT_UART0 + VIRT_UART0_SIZE).into(),
            MapType::Identical,
            MapPerm::R | MapPerm::W,
            None,
        );

        // VIRT_UART0_VIRTIO
        mm.insert_area(
            VIRT_UART_VIRTIO.into(),
            (VIRT_UART_VIRTIO + VIRT_UART_VIRTIO_SIZE).into(),
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
                        let start_va = VirtAddr(ph.virtual_addr() as usize);
                        let end_va = VirtAddr((ph.virtual_addr() + ph.mem_size()) as usize);
                        // println!(
                        //     "i = {}, start_vpn = {:?}, end_vpn = {:?}, start_va = {:?}, \
                        // end_va = {:?}",
                        //     i,
                        //     start_va.floor(),
                        //     end_va.ceil(),
                        //     start_va,
                        //     end_va
                        // );
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
                        mm.insert_area(
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
        let user_stack_bottom: usize = VirtAddr::from(max_end_va.ceil()).0 + PAGE_SIZE;

        mm.user_stack_bottom = user_stack_bottom;
        mm.entry_point = elf.header.pt2.entry_point() as usize;
        mm
    }

    /// Clone a memory manager. For fork.
    pub fn from_existed(another_mm: &Self) -> Self {
        let mut mm = MemoryManager::empty();
        mm.entry_point = another_mm.entry_point;
        mm.user_stack_bottom = another_mm.user_stack_bottom;
        mm.map_trampoline();
        for (_, area) in another_mm.areas.iter() {
            mm.insert_area(
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
            VirtAddr::from(TRAMPOLINE).into(),
            PhysAddr::from(strampoline as usize).into(),
            PTEFlags::R | PTEFlags::X,
        )
    }
    pub fn insert_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPerm,
        data: Option<&[u8]>,
    ) {
        // println!(
        //     "[kernel] push_area: token = {:#x}, start_va = {:#x}, end_va = {:#x}",
        //     self.page_table.token(),
        //     start_va.0,
        //     end_va.0
        // );
        let frame_guards = self.get_area_frame_guards(start_va, end_va, map_type, map_perm);
        let area = Area::new(start_va, end_va, map_type, map_perm, frame_guards);
        self.areas.insert(start_va.0, area);
        if let Some(data) = data {
            self.write(start_va, data);
        }
    }
    /// release virtual memory area
    pub fn remove_area(&mut self, start_va: VirtAddr) {
        // println!(
        //     "[kernel] remove_area: token = {:#x}, start_va = {:#x}",
        //     self.page_table.token(),
        //     start_va.0
        // );
        let area = self.areas.remove(&start_va.0).unwrap();
        for vpn in NumRange::new(area.start_vpn, area.end_vpn) {
            self.page_table.unmap(vpn);
        }
    }

    #[no_mangle]
    pub fn activate(&self) {
        // println!("activate satp = {:#x}", self.page_table.token());
        let satp = self.page_table.token();
        unsafe {
            satp::write(satp);
            asm!("sfence.vma");
        }
    }
    /// get data from the specified virtual address and length
    pub fn read(&self, start_va: VirtAddr, len: usize) -> Vec<u8> {
        let mut cur_vpn = start_va.floor();
        let mut cur_start = start_va.get_page_offset();
        let mut data = Vec::new();
        loop {
            let cur_end = min(cur_start + len - data.len(), PAGE_SIZE);
            let src = &self
                .page_table
                .find_pte(cur_vpn)
                .unwrap()
                .ppn()
                .get_bytes_array()[cur_start..cur_end];
            data.extend_from_slice(src);
            if data.len() >= len {
                break;
            }
            cur_vpn.0 += 1;
            cur_start = 0;
        }
        data
    }

    pub fn read_str(&self, start_va: VirtAddr) -> String {
        let mut data = String::new();
        let mut va = start_va;
        loop {
            let c: u8 = self
                .page_table
                .find_pte(va.floor())
                .unwrap()
                .ppn()
                .get_bytes_array()[va.get_page_offset()];
            if c == 0 {
                break;
            }
            data.push(c as char);
            va.0 += 1;
        }
        data
    }

    /// copy data to the specified virtual address
    pub fn write(&self, start_va: VirtAddr, data: &[u8]) {
        let mut cur_dst_vpn = start_va.floor();
        let mut cur_src_start: usize = 0;
        let mut cur_dst_start = start_va.get_page_offset();
        let end: usize = data.len();
        loop {
            let cur_src_end = min(
                cur_src_start + min(PAGE_SIZE - cur_dst_start, PAGE_SIZE),
                end
            );
            let src = &data[cur_src_start..cur_src_end];
            let dst = &mut self
                .page_table
                .find_pte(cur_dst_vpn)
                .unwrap()
                .ppn()
                .get_bytes_array()[cur_dst_start..cur_dst_start + src.len()];
            dst.copy_from_slice(src);
            cur_src_start += src.len();
            if cur_src_start >= end {
                break;
            }
            cur_dst_start += src.len();
            if cur_dst_start >= PAGE_SIZE {
                assert_eq!(cur_dst_start, PAGE_SIZE);
                cur_dst_vpn.0 += 1;
                cur_dst_start = 0;
            }
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
