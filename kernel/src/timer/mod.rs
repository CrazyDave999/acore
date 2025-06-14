use crate::config::*;
use crate::proc::{wakeup_thread, ThreadControlBlock};
use crate::sync::UPSafeCell;
use alloc::collections::BinaryHeap;
use alloc::sync::Arc;
use core::arch::global_asm;
use lazy_static::lazy_static;
use riscv::register::{mie, mscratch, mstatus, mtvec};

global_asm!(include_str!("mtime_trap.S"));

const CLOCK_FREQ: usize = 12500000; // Hz, 一秒内mtime的增量
#[allow(unused)]
const MICRO_PER_SEC: usize = 1_000_000;
#[allow(unused)]
const MILLI_PER_SEC: usize = 1_000;

const TICKS_PER_SEC: usize = 100;

pub fn set_time_cmp(time: usize) {
    unsafe {
        (MTIMECMP as *mut usize).write_volatile(time);
    }
}

pub fn get_time() -> usize {
    unsafe { (MTIME as *const usize).read_volatile() }
}

#[allow(unused)]
pub fn get_time_us() -> usize {
    get_time() / (CLOCK_FREQ / MICRO_PER_SEC)
}

#[allow(unused)]
pub fn get_time_ms() -> usize {
    get_time() / (CLOCK_FREQ / MILLI_PER_SEC)
}

pub fn set_next_trigger() {
    set_time_cmp(get_time() + 1 * CLOCK_FREQ / TICKS_PER_SEC);
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

pub struct TimerCondVar {
    pub expire_ms: usize,
    pub tcb: Arc<ThreadControlBlock>,
}

impl PartialEq for TimerCondVar {
    fn eq(&self, other: &Self) -> bool {
        self.expire_ms == other.expire_ms
    }
}
impl Eq for TimerCondVar {}
impl PartialOrd for TimerCondVar {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        let a = -(self.expire_ms as isize);
        let b = -(other.expire_ms as isize);
        Some(a.cmp(&b))
    }
}
impl Ord for TimerCondVar {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}
lazy_static! {
    static ref TIMERS: UPSafeCell<BinaryHeap<TimerCondVar>> =
        unsafe { UPSafeCell::new(BinaryHeap::<TimerCondVar>::new()) };
}

pub fn create_timer(expire_ms: usize, tcb: Arc<ThreadControlBlock>) {
    let mut timers = TIMERS.exclusive_access();
    timers.push(TimerCondVar {
        expire_ms,
        tcb: tcb,
    });
}

pub fn remove_timer(tcb: Arc<ThreadControlBlock>) {
    let mut timers = TIMERS.exclusive_access();
    let mut new_timers = BinaryHeap::<TimerCondVar>::new();
    for condvar in timers.drain() {
        if Arc::as_ptr(&tcb) != Arc::as_ptr(&condvar.tcb) {
            new_timers.push(condvar);
        }
    }
    timers.clear();
    timers.append(&mut new_timers);
}

/// Wakeup threads whose timers have expired and update the TIMERS
pub fn check_timer() {
    let cur_ms = get_time_ms();
    let mut timers = TIMERS.exclusive_access();
    while let Some(timer) = timers.peek() {
        if timer.expire_ms <= cur_ms {
            wakeup_thread(Arc::clone(&timer.tcb));
            timers.pop();
        } else {
            break;
        }
    }
}
