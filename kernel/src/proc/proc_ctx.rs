use crate::trap::trap_return;

/// Information for switch
pub struct ProcContext {
    pub ra: usize,
    pub sp: usize,
    pub s: [usize; 12],
}

impl ProcContext {
    pub fn empty() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }
    /// for newly created pcb
    pub fn new(sp: usize) -> Self {
        Self {
            ra: trap_return as usize,
            sp,
            s: [0; 12],
        }
    }
}