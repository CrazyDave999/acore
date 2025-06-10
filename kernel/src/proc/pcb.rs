use crate::config::TRAP_CONTEXT;
use crate::fs::kernel_file::{KernelFile, OpenFlags};
use crate::fs::stdio::{Stdin, Stdout};
use crate::fs::File;
use crate::mm::{get_kernel_stack_info, KERNEL_MM};
use crate::mm::{init_kernel_stack, release_kernel_stack, VirtAddr};
use crate::mm::{MemoryManager, PhysPageNum};
use crate::proc::pid::{pid_alloc, PIDGuard};
use crate::proc::proc_ctx::ProcContext;
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::sync::Weak;
use alloc::vec::Vec;
use core::cell::RefMut;
use lazy_static::lazy_static;

#[derive(Debug)]
pub enum ProcessState {
    Ready,
    Running,
    // Blocked,
    Zombie,
}

pub struct ProcessControlBlock {
    pub pid: PIDGuard,
    pub inner: UPSafeCell<ProcessControlBlockInner>,
}

pub struct ProcessControlBlockInner {
    pub state: ProcessState,
    pub trap_ctx_ppn: PhysPageNum,
    pub proc_ctx: ProcContext,
    pub parent: Option<Weak<ProcessControlBlock>>,
    pub children: Vec<Arc<ProcessControlBlock>>,
    pub exit_code: i32,
    pub mm: MemoryManager,
    pub fd_table: FileDescriptorTable,
}

impl ProcessControlBlock {
    pub fn from_elf(data: &[u8]) -> Self {
        let mm = MemoryManager::from_elf(data);
        let trap_ctx_ppn = mm
            .page_table
            .find_pte(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let pid_guard = pid_alloc();
        let (_, kernel_stack_top) = init_kernel_stack(pid_guard.0);

        let trap_ctx: &mut TrapContext = trap_ctx_ppn.get_mut();
        *trap_ctx = TrapContext::app_init_context(
            mm.entry_point,
            mm.user_stack_top,
            KERNEL_MM.exclusive_access().page_table.token(),
            kernel_stack_top,
        );

        let pcb_inner = ProcessControlBlockInner {
            state: ProcessState::Ready,
            trap_ctx_ppn,
            proc_ctx: ProcContext::new(kernel_stack_top),
            parent: None,
            children: Vec::new(),
            exit_code: 0,
            mm,
            fd_table: FileDescriptorTable::new(),
        };

        ProcessControlBlock {
            pid: pid_guard,
            inner: unsafe { UPSafeCell::new(pcb_inner) },
        }
    }
    pub fn token(&self) -> usize {
        self.inner.exclusive_access().mm.page_table.token()
    }
    pub fn exclusive_access(&self) -> RefMut<'_, ProcessControlBlockInner> {
        self.inner.exclusive_access()
    }
    pub fn getpid(&self) -> usize {
        self.pid.0
    }
    pub fn fork(self: &Arc<Self>) -> Arc<Self> {
        let mut parent_inner = self.exclusive_access();
        let mm = MemoryManager::from_existed(&parent_inner.mm);
        let trap_ctx_ppn = mm
            .page_table
            .find_ppn(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap();
        let pid_guard = pid_alloc();
        // println!("new pid: {}", pid_guard.0);
        let (_, kernel_stack_top) = init_kernel_stack(pid_guard.0);
        trap_ctx_ppn.get_mut::<TrapContext>().kernel_sp = kernel_stack_top;

        let pcb = Arc::new(ProcessControlBlock {
            pid: pid_guard,
            inner: unsafe {
                UPSafeCell::new(ProcessControlBlockInner {
                    state: ProcessState::Ready,
                    trap_ctx_ppn,
                    proc_ctx: ProcContext::new(kernel_stack_top),
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    mm,
                    fd_table: FileDescriptorTable::new(),
                })
            },
        });

        parent_inner.children.push(Arc::clone(&pcb));
        pcb
    }
    pub fn exec(&self, data: &[u8]) {
        let mm = MemoryManager::from_elf(data);
        let trap_ctx_ppn = mm
            .page_table
            .find_ppn(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap();
        let mut inner = self.exclusive_access();
        // abandon old virtual space and take new one, so just substitute
        inner.mm = mm;
        inner.trap_ctx_ppn = trap_ctx_ppn;
        let trap_ctx = trap_ctx_ppn.get_mut::<TrapContext>();

        let (_, kernel_stack_top) = get_kernel_stack_info(self.getpid());

        *trap_ctx = TrapContext::app_init_context(
            inner.mm.entry_point,
            inner.mm.user_stack_top,
            KERNEL_MM.exclusive_access().page_table.token(),
            kernel_stack_top,
        );
    }
    pub fn is_zombie(&self) -> bool {
        match self.inner.exclusive_access().state {
            ProcessState::Zombie => true,
            _ => false,
        }
    }
    pub fn set_state(&self, state: ProcessState) {
        self.inner.exclusive_access().state = state;
    }
}

impl Drop for ProcessControlBlock {
    fn drop(&mut self) {
        release_kernel_stack(self.pid.0);
    }
}

impl ProcessControlBlockInner {
    pub fn get_file(&self, fd: usize) -> Option<Arc<dyn File + Send + Sync>> {
        self.fd_table.get_file(fd)
    }
}

lazy_static! {
    pub static ref INIT_PCB: Arc<ProcessControlBlock> = Arc::new({
        let kernel_file = KernelFile::from_path("/init", OpenFlags::RDONLY).unwrap();
        let v = kernel_file.read_all();
        ProcessControlBlock::from_elf(v.as_slice())
    });
}

pub struct FileDescriptorTable {
    fd_table: BTreeMap<usize, Arc<dyn File + Send + Sync>>,
    recycled: Vec<usize>,
}
impl FileDescriptorTable {
    pub fn new() -> Self {
        FileDescriptorTable {
            fd_table: BTreeMap::from([
                (0, Arc::new(Stdin) as Arc<dyn File + Send + Sync>),
                (1, Arc::new(Stdout)),
                (2, Arc::new(Stdout)),
            ]),
            recycled: Vec::new(),
        }
    }
    pub fn insert_kernel_file(&mut self, kernel_file: Arc<KernelFile>) -> isize {
        let fd = if let Some(fd) = self.recycled.pop() {
            fd
        } else {
            self.fd_table.len()
        };
        self.fd_table.insert(fd, kernel_file);
        fd as isize
    }
    pub fn dealloc_fd(&mut self, fd: usize) -> isize {
        if let Some(_) = self.fd_table.remove(&fd) {
            self.recycled.push(fd);
            0
        } else {
            // panic!("fd {} not found", fd);
            -1
        }
    }
    pub fn get_file(&self, fd: usize) -> Option<Arc<dyn File + Send + Sync>> {
        self.fd_table.get(&fd).map(|f| Arc::clone(f))
    }
}
