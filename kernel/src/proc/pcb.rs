use crate::fs::kernel_file::{KernelFile, OpenFlags};
use crate::fs::stdio::{Stdin, Stdout};
use crate::fs::File;
use crate::mm::MemoryManager;
use crate::mm::VirtAddr;
use crate::mm::KERNEL_MM;
use crate::proc::action::SignalActions;
use crate::proc::manager::insert_to_pid2pcb;
use crate::proc::resource::{pid_alloc, PIDGuard, RecycleAllocator};
use crate::proc::thread::ThreadControlBlock;
use crate::proc::{push_thread, SignalFlags};
use crate::sync::{Mutex, UPSafeCell};
use crate::trap::TrapContext;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::sync::Weak;
use alloc::vec::Vec;
use core::cell::RefMut;
use lazy_static::lazy_static;

#[derive(Debug)]
pub enum ProcessState {
    Ready,
    Zombie,
}

pub struct ProcessControlBlock {
    // immutable
    pub pid: PIDGuard,
    // mutable
    pub inner: UPSafeCell<ProcessControlBlockInner>,
}

pub struct ProcessControlBlockInner {
    pub state: ProcessState,

    pub parent: Option<Weak<ProcessControlBlock>>,
    pub children: Vec<Arc<ProcessControlBlock>>,
    pub exit_code: i32,
    pub mm: MemoryManager,
    pub fd_table: FileDescriptorTable,
    pub signals: SignalFlags,
    pub signal_mask: SignalFlags,
    // the signal which is being handling
    pub handling_sig: isize,
    // Signal actions
    pub signal_actions: SignalActions,
    // if the task is killed
    pub killed: bool,
    // if the task is frozen by a signal
    pub frozen: bool,
    // backup of the trap context when handling a signal
    pub trap_ctx_backup: Option<TrapContext>,
    pub threads: Vec<Option<Arc<ThreadControlBlock>>>,
    // unified allocator for thread resources
    pub thread_res_allocator: RecycleAllocator,
    pub mutex_list: Vec<Option<Arc<dyn Mutex>>>
}

impl ProcessControlBlock {
    pub fn from_elf(data: &[u8]) -> Arc<Self> {
        let mm = MemoryManager::from_elf(data);
        let entry_point = mm.entry_point;
        let proc_user_stack_bottom = mm.user_stack_bottom;

        let pid_guard = pid_alloc();

        let proc = Arc::new(Self {
            pid: pid_guard,
            inner: unsafe {
                UPSafeCell::new(ProcessControlBlockInner {
                    state: ProcessState::Ready,
                    parent: None,
                    children: Vec::new(),
                    exit_code: 0,
                    mm,
                    fd_table: FileDescriptorTable::new(),
                    signals: SignalFlags::empty(),
                    signal_mask: SignalFlags::empty(),
                    handling_sig: -1,
                    signal_actions: SignalActions::default(),
                    killed: false,
                    frozen: false,
                    trap_ctx_backup: None,
                    threads: Vec::new(),
                    thread_res_allocator: RecycleAllocator::new(),
                    mutex_list: Vec::new(),
                })
            },
        });

        // create a main thread
        let thread = Arc::new(ThreadControlBlock::new(
            Arc::clone(&proc),
            proc_user_stack_bottom,
            true,
        ));

        // set trap ctx for the main thread
        let thr_inner = thread.exclusive_access();
        let trap_ctx = thr_inner.trap_ctx_ppn.get_mut::<TrapContext>();
        let user_stack_top = thr_inner.res.as_ref().unwrap().get_user_stack_top();
        let kernel_stack_top = thread.kernel_stack.get_top();
        drop(thr_inner);
        *trap_ctx = TrapContext::app_init_context(
            entry_point,
            user_stack_top,
            KERNEL_MM.exclusive_access().page_table.token(),
            kernel_stack_top,
        );

        // add main thread to the process
        let mut proc_inner = proc.exclusive_access();
        proc_inner.threads.push(Some(thread.clone()));
        drop(proc_inner);

        proc
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
        assert_eq!(
            parent_inner.thread_count(),
            1,
            "fork only supports single-threaded process"
        );
        let mm = MemoryManager::from_existed(&parent_inner.mm);
        let pid_guard = pid_alloc();
        // println!("new pid: {}", pid_guard.0);

        let pcb = Arc::new(ProcessControlBlock {
            pid: pid_guard,
            inner: unsafe {
                UPSafeCell::new(ProcessControlBlockInner {
                    state: ProcessState::Ready,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    mm,
                    fd_table: parent_inner.fd_table.clone(),
                    signals: SignalFlags::empty(),
                    // inherit parent's signal mask and signal actions
                    signal_mask: parent_inner.signal_mask,
                    handling_sig: -1,
                    signal_actions: parent_inner.signal_actions.clone(),
                    killed: false,
                    frozen: false,
                    trap_ctx_backup: None,
                    threads: Vec::new(),
                    thread_res_allocator: RecycleAllocator::new(),
                    mutex_list: Vec::new(),
                })
            },
        });

        parent_inner.children.push(Arc::clone(&pcb));

        // create main thread for the child proc
        let tcb = Arc::new(ThreadControlBlock::new(
            Arc::clone(&pcb),
            parent_inner
                .get_thread(0)
                .exclusive_access()
                .res
                .as_ref()
                .unwrap()
                .user_stack_base,
            // no need to map again since the memory manager is cloned
            false,
        ));
        let mut child_inner = pcb.exclusive_access();
        child_inner.threads.push(Some(tcb.clone()));
        drop(child_inner);

        // modify kernel_stack_top
        let thr_inner = tcb.exclusive_access();
        let trap_ctx = thr_inner.get_trap_ctx();
        trap_ctx.kernel_sp = tcb.kernel_stack.get_top();
        drop(thr_inner);
        insert_to_pid2pcb(pcb.getpid(), pcb.clone());

        // add this thread to the scheduler
        push_thread(tcb.clone());

        pcb
    }
    pub fn exec(self: &Arc<Self>, data: &[u8], args: Vec<String>) {
        // println!("exec: pid {}, args: {:?}", self.getpid(), args);
        let mut proc_inner = self.exclusive_access();
        assert_eq!(
            proc_inner.thread_count(),
            1,
            "exec only supports single-threaded process"
        );
        let mm = MemoryManager::from_elf(data);
        let user_stack_bottom = mm.user_stack_bottom;
        let entry_point = mm.entry_point;

        // substitute memory manager
        proc_inner.mm = mm;

        // add the mapping for main thread again since mm has been changed
        let thread = proc_inner.get_thread(0);

        drop(proc_inner);

        let mut thr_inner = thread.exclusive_access();
        thr_inner.res.as_mut().unwrap().user_stack_base = user_stack_bottom;
        thr_inner.res.as_mut().unwrap().add_mapping();
        thr_inner.trap_ctx_ppn = thr_inner.res.as_mut().unwrap().get_trap_ctx_ppn();

        // push args to user stack

        // prepare argv
        let mut user_sp = thr_inner.res.as_mut().unwrap().get_user_stack_top();

        user_sp -= (args.len() + 1) * core::mem::size_of::<usize>();
        let argv_base = user_sp;

        let mut proc_inner = self.exclusive_access();

        let mm = &mut proc_inner.mm;

        // null terminator for argv
        mm.write(
            VirtAddr::from(argv_base + args.len() * core::mem::size_of::<usize>()),
            &[0u8; core::mem::size_of::<usize>()],
        );

        for i in 0..args.len() {
            user_sp -= args[i].len() + 1;
            mm.write(
                VirtAddr::from(argv_base + i * core::mem::size_of::<usize>()),
                user_sp.to_le_bytes().as_slice(),
            );
            mm.write(
                VirtAddr::from(user_sp),
                args[i]
                    .as_bytes()
                    .iter()
                    .chain(&[0u8])
                    .copied()
                    .collect::<Vec<u8>>()
                    .as_slice(),
            );
        }

        // align to 8 bytes
        user_sp -= user_sp % core::mem::size_of::<usize>();

        // init trap ctx
        let mut trap_ctx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_MM.exclusive_access().page_table.token(),
            thread.kernel_stack.get_top(),
        );

        trap_ctx.x[10] = args.len();
        trap_ctx.x[11] = argv_base;
        *thr_inner.get_trap_ctx() = trap_ctx;
    }
    pub fn is_zombie(&self) -> bool {
        match self.inner.exclusive_access().state {
            ProcessState::Zombie => true,
            _ => false,
        }
    }
    // pub fn set_state(&self, state: ProcessState) {
    //     self.inner.exclusive_access().state = state;
    // }
}

impl ProcessControlBlockInner {
    pub fn get_file(&self, fd: usize) -> Option<Arc<dyn File + Send + Sync>> {
        self.fd_table.get_file(fd)
    }
    pub fn alloc_tid(&mut self) -> usize {
        self.thread_res_allocator.alloc()
    }
    pub fn dealloc_tid(&mut self, tid: usize) {
        self.thread_res_allocator.dealloc(tid);
    }
    pub fn thread_count(&self) -> usize {
        self.threads.len()
    }
    pub fn get_thread(&self, tid: usize) -> Arc<ThreadControlBlock> {
        self.threads[tid]
            .as_ref()
            .expect("thread not found")
            .clone()
    }
}

lazy_static! {
    pub static ref INIT_PCB: Arc<ProcessControlBlock> = {
        let kernel_file = KernelFile::from_path("/init", OpenFlags::RDONLY).unwrap();
        let v = kernel_file.read_all();
        ProcessControlBlock::from_elf(v.as_slice())
    };
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
    pub fn clone(&self) -> Self {
        FileDescriptorTable {
            fd_table: self.fd_table.clone(),
            recycled: self.recycled.clone(),
        }
    }
    pub fn insert_file(&mut self, file: Arc<dyn File + Send + Sync>) -> isize {
        let fd = if let Some(fd) = self.recycled.pop() {
            fd
        } else {
            self.fd_table.len()
        };
        self.fd_table.insert(fd, file);
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
    pub fn clear(&mut self) {
        self.fd_table.clear();
        self.recycled.clear();
    }
}
