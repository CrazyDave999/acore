//! The panic handler

use core::{arch::asm, panic::PanicInfo};
use crate::config::*;

#[panic_handler]
/// panic handler
fn panic(info: &PanicInfo) -> ! {
    unreachable!()
}

pub fn shutdown() -> ! {
    unsafe {
        asm!(
            "sw {0}, 0({1})",
            in(reg) FINISHER_PASS,
            in(reg) VIRT_TEST
        );
    }
    panic!("[kernel] Fail to shutdown.");
}
