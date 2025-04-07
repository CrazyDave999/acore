use core::arch::global_asm;
use riscv::register::{mie, mscratch, mstatus, mtvec};
use crate::config::*;

global_asm!(include_str!("mtime_trap.S"));

const CLOCK_FREQ: usize = 12500000; // Hz, 一秒内mtime的增量
const MICRO_PER_SEC:usize = 1_000_000;
const MILLI_PER_SEC:usize = 1_000;

const TICKS_PER_SEC: usize = 100;

pub fn set_timer(time: usize) {
    unsafe {
        (MTIMECMP as *mut usize).write_volatile(time);
    }
}

pub fn get_time() -> usize {
    unsafe { (MTIME as *const usize).read_volatile() }
}

pub fn get_time_us() -> usize {
    get_time() / (CLOCK_FREQ / MICRO_PER_SEC)
}

pub fn get_time_ms() -> usize {
    get_time() / (CLOCK_FREQ / MILLI_PER_SEC)
}

pub fn set_next_trigger() {
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
}


#[link_section = ".bss.stack"]
#[no_mangle]
pub static mut TIMER_SCRATCH: [usize; 5] = [0; 5];

pub fn init() {
    extern "C" {
        fn __mtime_trap();
    }
    unsafe {
        mtvec::write(__mtime_trap as usize, mtvec::TrapMode::Direct);

        // let scratch = &mut TIMER_SCRATCH;
        TIMER_SCRATCH[3] = MTIMECMP;
        TIMER_SCRATCH[4] = CLOCK_FREQ / TICKS_PER_SEC;
        mscratch::write(TIMER_SCRATCH.as_mut_ptr() as usize);

        mstatus::set_mie();

        mie::set_mtimer();
    }
}