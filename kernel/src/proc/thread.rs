use crate::mm::PhysPageNum;
use crate::proc::ctx::ThreadContext;
use crate::proc::pcb::ProcessControlBlock;
use crate::proc::resource::{kernel_stack_alloc, KernelStackGuard, ThreadResource};
use crate::sync::UPSafeCell;
use alloc::sync::{Arc, Weak};
use core::cell::RefMut;
use crate::trap::TrapContext;

#[derive(Copy, Clone, PartialEq)]
pub enum ThreadState {
    Ready,
    Running,
    Blocked,
}

pub struct ThreadControlBlock {
    pub pcb: Weak<ProcessControlBlock>,
    pub kernel_stack: KernelStackGuard,

    inner: UPSafeCell<ThreadControlBlockInner>,
}
pub struct ThreadControlBlockInner {
    pub res: Option<ThreadResource>,
    pub trap_ctx_ppn: PhysPageNum,
    pub thread_ctx: ThreadContext,
    pub state: ThreadState,
    pub exit_code: Option<i32>,
}

impl ThreadControlBlockInner {
    pub fn get_trap_ctx(&self) -> &'static mut TrapContext {
        self.trap_ctx_ppn.get_mut()
    }
}
impl ThreadControlBlock {
    pub fn new(pcb: Arc<ProcessControlBlock>, user_stack_base: usize, add_map: bool) -> Self {
        let res = ThreadResource::new(pcb.clone(), user_stack_base, add_map);
        let trap_ctx_ppn = res.get_trap_ctx_ppn();
        let kernel_stack_guard = kernel_stack_alloc();
        let kernel_stack_top = kernel_stack_guard.get_top();
        ThreadControlBlock {
            pcb: Arc::downgrade(&pcb),
            kernel_stack: kernel_stack_guard,
            inner: unsafe {
                UPSafeCell::new(ThreadControlBlockInner {
                    res: Some(res),
                    trap_ctx_ppn,
                    thread_ctx: ThreadContext::new(kernel_stack_top),
                    state: ThreadState::Ready,
                    exit_code: None,
                })
            },
        }
    }
    pub fn exclusive_access(&self) -> RefMut<'_, ThreadControlBlockInner> {
        self.inner.exclusive_access()
    }
}
