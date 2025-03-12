#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

extern crate bitflags;
extern crate alloc;

use log::*;
use riscv::register::{mstatus, mepc, satp, pmpaddr0,pmpcfg0};

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
unsafe fn from_m_to_s() {
    // mstatus set for privilege change, mepc set for correct jumping
    mstatus::set_mpp(riscv::register::mstatus::MPP::Supervisor);
    mepc::write(rust_main as usize);

    // disable page table for the supervisor mode
    satp::write(0);

    pmpaddr0::write(0x3fffffffffffffusize);
    pmpcfg0::write(0xf);


}

fn rust_init() {
    clear_bss();
    UART.init();
    console::logging::init();

}

#[no_mangle]
pub fn rust_main() -> ! {
    rust_init();
    println!("Hello from CrazyDave's acore implementation.");
    info!("Let's go!");
    console::shutdown();
}
