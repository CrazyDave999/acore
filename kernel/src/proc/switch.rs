

core::arch::global_asm!(include_str!("switch.S"));
use super::ctx::ThreadContext;
// switching from proc A to proc B just like calling __switch in proc A's kernel mode
extern "C" {
    pub fn __switch(cur_thr_ctx: *mut ThreadContext, next_thr_ctx: *const ThreadContext);
}
