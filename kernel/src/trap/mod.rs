mod context;

use crate::syscall::syscall;

use crate::config::*;
use crate::proc::{exit_proc, get_cur_trap_ctx, get_cur_user_token, switch_proc};
use core::arch::{asm, global_asm};
use log::error;
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    sie, sip, stval, stvec,
};

global_asm!(include_str!("trampoline.S"));

/// initialize CSR `stvec` as the entry of `__alltraps`
pub fn init() {
    extern "C" {
        fn __alltraps();
    }
    unsafe {
        set_kernel_trap_entry();
        // sstatus::set_sie();
        sie::set_sext();
        sie::set_stimer();
        sie::set_ssoft();
    }
}
fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(trap_from_kernel as usize, TrapMode::Direct);
    }
}
fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE, TrapMode::Direct);
    }
}

#[no_mangle]
/// handle an interrupt, exception, or system call from user space
pub fn trap_handler() -> ! {
    set_kernel_trap_entry();

    let scause = scause::read();
    let stval = stval::read();
    let ctx = get_cur_trap_ctx();

    // println!(
    //     "[kernel] trap_handler, scauce = {:?}, stval = {:#x}",
    //     scause.cause(),
    //     stval
    // );
    match scause.cause() {
        Trap::Interrupt(Interrupt::SupervisorSoft) => {
            // delegated by m mode, actually a machine timer interrupt
            // println!("[kernel] FUCK! TIME INTERRUPT! {}", stval);
            let sip = sip::read().bits();
            unsafe {
                asm! {"csrw sip, {sip}", sip = in(reg) sip ^ 2};
            }
            set_next_trigger();
            switch_proc();
        }
        Trap::Exception(Exception::UserEnvCall) => {
            ctx.sepc += 4;
            ctx.x[10] = syscall(ctx.x[17], [ctx.x[10], ctx.x[11], ctx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault) => {
            error!(
                "[kernel] PageFault, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it\
                .\n",
                stval, ctx.sepc
            );
            exit_proc(-2);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            error!("IllegalInstruction in application\n");
            exit_proc(-3);
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    trap_return()
}

#[no_mangle]
pub fn trap_return() -> ! {
    // println!("[kernel] trap_return: pid: {}", get_cur_proc().unwrap().pid.0);
    set_user_trap_entry();
    let trap_ctx_ptr = TRAP_CONTEXT;
    let user_satp = get_cur_user_token();
    // println!("satp = {:#x}", user_satp);
    extern "C" {
        fn __alltraps();
        fn __restore();
    }
    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
    unsafe {
        asm!(
        "fence.i",
        "jr {restore_va}",
        restore_va = in(reg) restore_va,
        in("a0") trap_ctx_ptr,
        in("a1") user_satp,
        options(noreturn)
        )
    }
}

#[no_mangle]
pub fn trap_from_kernel() -> ! {
    panic!(
        "a trap {:?} happened in kernel mode!",
        scause::read().cause()
    );
}

use crate::timer::set_next_trigger;
pub use context::TrapContext;
