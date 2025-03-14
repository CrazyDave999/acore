#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

extern crate bitflags;
extern crate alloc;

use core::arch::asm;
use log::*;
use riscv::register::{mstatus, mepc, satp, pmpaddr0, pmpcfg0};

mod config;
mod console;
mod mm;
mod sync;
mod syscall;
mod timer;
mod trap;
mod proc;

use console::mmio::UART;

core::arch::global_asm!(include_str!("entry.asm"));

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
unsafe fn from_m_to_s() {
    // mstatus set for privilege change, mepc set for correct jumping
    mstatus::set_mpp(riscv::register::mstatus::MPP::Supervisor);
    mepc::write(rust_main as usize);

    // disable page table for the supervisor mode
    satp::write(0);

    pmpaddr0::write(0x3fffffffffffffusize);
    pmpcfg0::write(0xf);

    // keep CPU's hartid in tp register
    asm!("csrr tp, mhartid");

    asm!(
        "csrw mideleg, {mideleg}", // some bits could not be set by this method
        "csrw medeleg, {medeleg}",
        "mret",
        medeleg = in(reg) !0,
        mideleg = in(reg) !0,
        options(noreturn),
    );
}

fn rust_init() {
    clear_bss();
    UART.init();
    console::logging::init();
    mm::init();
}

#[no_mangle]
pub fn rust_main() -> ! {
    rust_init();
    println!("Hello from CrazyDave's acore implementation.");
    info!("Let's go!");
    mm::buddy::test_vec();
    mm::buddy::test_btree_map();
    console::shutdown();
}
