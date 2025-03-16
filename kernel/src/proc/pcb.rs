use alloc::rc::Weak;
use alloc::sync::Arc;
use alloc::vec::Vec;

pub enum ProcessState {
    Ready,
    Running,
    Blocked,
    // Zombie,
}
pub struct ProcessControlBlock {
    pid: usize,
    state: ProcessState,
    parent: Option<Weak<ProcessControlBlock>>,
    children: Vec<Arc<ProcessControlBlock>>,
}