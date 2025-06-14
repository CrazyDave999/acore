use crate::config::{KERNEL_STACK_SIZE, PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT_BASE, USER_STACK_SIZE};
use crate::mm::{MapPerm, MapType, PhysPageNum, VirtAddr, KERNEL_MM};
use crate::proc::pcb::ProcessControlBlock;
use crate::sync::UPSafeCell;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use lazy_static::lazy_static;
use crate::println;

pub struct RecycleAllocator {
    cap: usize,
    recycled: Vec<usize>,
}

impl RecycleAllocator {
    pub fn new() -> Self {
        RecycleAllocator {
            cap: 0,
            recycled: Vec::new(),
        }
    }
    pub fn alloc(&mut self) -> usize {
        if let Some(id) = self.recycled.pop() {
            id
        } else {
            let id = self.cap;
            self.cap += 1;
            id
        }
    }
    pub fn dealloc(&mut self, id: usize) {
        assert!(id < self.cap);
        assert!(
            self.recycled.iter().all(|&p| p != id),
            "id {} has been recycled",
            id
        );
        self.recycled.push(id);
    }
}

lazy_static! {
    pub static ref PID_ALLOCATOR: UPSafeCell<RecycleAllocator> =
        unsafe { UPSafeCell::new(RecycleAllocator::new()) };
}
#[derive(Debug)]
pub struct PIDGuard(pub usize);

impl Drop for PIDGuard {
    fn drop(&mut self) {
        PID_ALLOCATOR.exclusive_access().dealloc(self.0);
    }
}

pub fn pid_alloc() -> PIDGuard {
    PIDGuard(PID_ALLOCATOR.exclusive_access().alloc())
}

fn get_trap_ctx_addr_by_tid(tid: usize) -> usize {
    TRAP_CONTEXT_BASE - tid * PAGE_SIZE
}

fn get_user_stack_bottom_by_tid(user_stack_base: usize, tid: usize) -> usize {
    user_stack_base + tid * (PAGE_SIZE + USER_STACK_SIZE)
}

pub struct ThreadResource {
    pub tid: usize,
    pub user_stack_base: usize,
    pub pcb: Weak<ProcessControlBlock>,
}
impl ThreadResource {
    /// when forking, there is no need to add same mapping again, then add_map can be false
    pub fn new(pcb: Arc<ProcessControlBlock>, user_stack_base: usize, add_map: bool) -> Self {
        let tid = pcb.exclusive_access().alloc_tid();
        let thr_res = ThreadResource {
            tid,
            user_stack_base,
            pcb: Arc::downgrade(&pcb),
        };
        if add_map {
            thr_res.add_mapping();
        }
        thr_res
    }
    /// actually map the user stack and trampoline to user addr space
    pub fn add_mapping(&self) {
        let proc = self.pcb.upgrade().unwrap();
        let mut inner = proc.exclusive_access();
        // map user stack
        let user_stack_bottom = get_user_stack_bottom_by_tid(self.user_stack_base, self.tid);
        let user_stack_top = user_stack_bottom + USER_STACK_SIZE;
        inner.mm.insert_area(
            VirtAddr::from(user_stack_bottom),
            VirtAddr::from(user_stack_top),
            MapType::Framed,
            MapPerm::R | MapPerm::W | MapPerm::U,
            None,
        );
        // map trampoline
        let trap_ctx_addr = get_trap_ctx_addr_by_tid(self.tid);
        inner.mm.insert_area(
            VirtAddr::from(trap_ctx_addr),
            VirtAddr::from(trap_ctx_addr + PAGE_SIZE),
            MapType::Framed,
            MapPerm::R | MapPerm::W,
            None,
        );
    }
    pub fn get_trap_ctx_ppn(&self) -> PhysPageNum {
        let pcb = self.pcb.upgrade().unwrap();
        let inner = pcb.exclusive_access();
        let trap_ctx_addr = get_trap_ctx_addr_by_tid(self.tid);
        inner.mm.page_table.find_ppn(
            VirtAddr::from(trap_ctx_addr).floor(),
        ).unwrap()
    }
    pub fn get_user_stack_top(&self) -> usize {
        get_user_stack_bottom_by_tid(self.user_stack_base, self.tid) + USER_STACK_SIZE
    }
}

impl Drop for ThreadResource {
    fn drop(&mut self) {
        // dealloc tid
        let proc = self.pcb.upgrade().unwrap();
        let mut inner = proc.exclusive_access();
        inner.dealloc_tid(self.tid);

        // remove user stack mapping
        let user_stack_bottom = get_user_stack_bottom_by_tid(self.user_stack_base, self.tid);
        inner.mm.remove_area(VirtAddr::from(user_stack_bottom));

        // remove trampoline mapping
        let trap_ctx_addr = get_trap_ctx_addr_by_tid(self.tid);
        inner.mm.remove_area(VirtAddr::from(trap_ctx_addr));
    }
}

lazy_static! {
    static ref KERNEL_STACK_ALLOCATOR: UPSafeCell<RecycleAllocator> =
        unsafe { UPSafeCell::new(RecycleAllocator::new()) };
}

/// kernel stack guard with kid
pub struct KernelStackGuard(pub usize);

impl KernelStackGuard {
    pub fn get_top(&self) -> usize {
        let (_, kernel_stack_top) = get_kernel_stack_info(self.0);
        kernel_stack_top
    }
}

/// Now the kernel stack is allocated by a unified kid, instead of pid or tid.
pub fn get_kernel_stack_info(kid: usize) -> (usize, usize) {
    let top = TRAMPOLINE - kid * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}

pub fn kernel_stack_alloc() -> KernelStackGuard {
    let kid = KERNEL_STACK_ALLOCATOR.exclusive_access().alloc();
    let (kernel_stack_bottom, kernel_stack_top) = get_kernel_stack_info(kid);
    KERNEL_MM.exclusive_access().insert_area(
        kernel_stack_bottom.into(),
        kernel_stack_top.into(),
        MapType::Framed,
        MapPerm::R | MapPerm::W,
        None,
    );
    KernelStackGuard(kid)
}

impl Drop for KernelStackGuard {
    fn drop(&mut self) {
        let (kernel_stack_bottom, _) = get_kernel_stack_info(self.0);
        KERNEL_MM.exclusive_access().remove_area(VirtAddr::from(kernel_stack_bottom));
        KERNEL_STACK_ALLOCATOR.exclusive_access().dealloc(self.0);
    }
}