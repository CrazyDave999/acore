//! The panic handler

use core::{arch::asm, panic::PanicInfo};
use crate::config::*;
use log::*;
use crate::println;

#[panic_handler]
/// panic handler
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        error!(
            "[kernel] Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    } else {
        error!("[kernel] Panicked: {}", info.message().unwrap());
    }
    shutdown()
}

pub fn shutdown() -> ! {
    println!("[kernel] Goodbye!");
    unsafe {
        asm!(
            "sw {0}, 0({1})",
            in(reg) FINISHER_PASS,
            in(reg) VIRT_TEST
        );
    }
    panic!("[kernel] Fail to shutdown.");
}
