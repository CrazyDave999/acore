
use riscv::register::sstatus::{self, Sstatus, SPP};
use crate::trap::trap_handler;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
/// trap context structure containing sstatus, sepc and registers
pub struct TrapContext {
    /// general regs[0..31]
    pub x: [usize; 32],
    /// CSR sstatus      
    pub sstatus: Sstatus,
    /// CSR sepc
    pub sepc: usize,
    /// Addr of Page Table
    pub kernel_satp: usize,
    /// kernel stack
    pub kernel_sp: usize,
    /// Addr of trap_handler function
    pub trap_handler: usize,
}

impl TrapContext {
    /// set stack pointer to x_2 reg (sp)
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }
    /// init app context
    pub fn app_init_context(
        entry: usize,
        sp: usize,
        kernel_satp: usize,
        kernel_sp: usize,
    ) -> Self {
        let mut sstatus = sstatus::read(); // CSR sstatus
        sstatus.set_spp(SPP::User); // previous privilege mode: user mode
        let mut ctx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry, // entry point of app
            kernel_satp,
            kernel_sp,
            trap_handler: trap_handler as usize,
        };
        ctx.set_sp(sp);
        ctx
    }
}