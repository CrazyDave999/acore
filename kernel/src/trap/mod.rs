//! Trap handling functionality
//!
//! For rCore, we have a single trap entry point, namely `__alltraps`. At
//! initialization in [`init()`], we set the `stvec` CSR to point to it.
//!
//! All traps go through `__alltraps`, which is defined in `trap.S`. The
//! assembly language code does just enough work restore the kernel space
//! context, ensuring that Rust code safely runs, and transfers control to
//! [`trap_handler()`].
//!
//! It then calls different functionality based on what exactly the exception
//! was. For example, timer interrupts trigger task preemption, and syscalls go
//! to [`syscall()`].

mod context;

use crate::syscall::syscall;

use crate::timer::set_next_trigger;
use core::arch::{asm, global_asm};
use riscv::register::{mtvec::TrapMode, scause::{self, Exception, Interrupt, Trap}, sie, sip, sstatus, stval, stvec};
pub use crate::println;

global_asm!(include_str!("trampoline.S"));

/// initialize CSR `stvec` as the entry of `__alltraps`
pub fn init() {
    extern "C" {
        fn __alltraps();
    }
    unsafe {
        stvec::write(__alltraps as usize, TrapMode::Direct);
        // sstatus::set_sie();
        sie::set_sext();
        sie::set_stimer();
        sie::set_ssoft();
    }
}

#[no_mangle]
/// handle an interrupt, exception, or system call from user space
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {

    let scause = scause::read();
    let stval = stval::read();

    // println!("trap_handler, scauce = {:?}, stval = {:#x}", scause.cause(), stval);
    match scause.cause() {
        Trap::Interrupt(Interrupt::SupervisorSoft) => {
            // delegated by m mode, actually a machine timer interrupt
            println!("FUCK! TIME INTERRUPT! {}", stval);
            let sip = sip::read().bits();
            unsafe {
                asm! {"csrw sip, {sip}", sip = in(reg) sip ^ 2};
            }
            // set_next_trigger();
        }
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            println!(
                "[kernel] PageFault in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.",
                stval, cx.sepc
            );
            panic!("PageFault in application");
            // exit_current_and_run_next();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            panic!("IllegalInstruction in application");
            // exit_current_and_run_next();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    cx
}

pub use context::TrapContext;
