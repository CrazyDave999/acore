use alloc::rc::Weak;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::RefCell;
use crate::mm::{MemoryManager, PhysPageNum};
use crate::proc::pid::{pid_alloc, PIDGuard};
use crate::proc::proc_ctx::ProcContext;

pub enum ProcessState {
    Ready,
    Running,
    Blocked,
    // Zombie,
}
pub struct ProcessControlBlock {
    pub pid: PIDGuard,
    pub inner: RefCell<ProcessControlBlockInner>,
}
pub struct ProcessControlBlockInner {
    pub state: RefCell<ProcessState>,
    pub trap_ctx_ppn: PhysPageNum,
    pub proc_ctx: ProcContext,
    pub parent: Option<Weak<ProcessControlBlock>>,
    pub children: Vec<Arc<ProcessControlBlock>>,
    pub exit_code: i32,
    pub mm: MemoryManager,
}

impl ProcessControlBlock {
    pub fn from_elf(data: &mut[u8]) -> Self {


    }
    pub fn token(&self) -> usize {
        self.inner.borrow().mm.page_table.token()
    }
}