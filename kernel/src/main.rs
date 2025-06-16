#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

extern crate bitflags;
extern crate alloc;

use core::arch::{asm, global_asm};
use riscv::register::{mstatus, mepc, satp, pmpaddr0, pmpcfg0};
use log::*;

mod config;
mod console;
mod mm;
mod sync;
mod syscall;
mod timer;
mod trap;
mod proc;
mod utils;
mod fs;
mod drivers;

use console::mmio::UART;
use crate::console::stdout::print;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.S"));

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    unsafe {
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize)
            .fill(0);
    }
}

#[no_mangle]
fn from_m_to_s() {
    // mstatus set for privilege change, mepc set for correct jumping
    unsafe {mstatus::set_mpp(mstatus::MPP::Supervisor);}
    mepc::write(rust_main as usize);

    // disable page table for the supervisor mode
    satp::write(0);

    pmpaddr0::write(0x3fffffffffffffusize);
    pmpcfg0::write(0xf);

    // keep CPU's hartid in tp register
    unsafe {
        asm!("csrr tp, mhartid");
    }
    timer::init();
    unsafe {
        asm!(
        "csrw mideleg, {mideleg}", // some bits could not be set by this method
        "csrw medeleg, {medeleg}",
        "mret",
        medeleg = in(reg) !0,
        mideleg = in(reg) !0,
        options(noreturn),
        );
    }
}

fn rust_init() {
    clear_bss();
    UART.init();
    console::logging::init();
    // println!("mm init start");
    mm::init();
    // println!("mm init ok");
    trap::init();
    timer::set_next_trigger();
}

#[no_mangle]
pub fn rust_main() {
    rust_init();
    println!("[kernel] Hello from kernel. This is CrazyDave's ACore implementation.");
    proc::launch(proc::INIT_PCB.clone());
}
