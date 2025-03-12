use super::pid::PIDGuard;

// stack of a process
pub struct ProcStack {
    pid: usize,
}

impl ProcStack {
    pub fn new(pid: usize) -> Self {
        Self { pid }
    }
}
