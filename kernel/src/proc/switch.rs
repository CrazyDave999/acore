

core::arch::global_asm!(include_str!("switch.S"));
use super::proc_ctx::ProcContext;
// switching from proc A to proc B just like calling __switch in proc A's kernel mode
extern "C" {
    pub fn __switch(cur_proc_ctx: *mut ProcContext, next_proc_ctx: *const ProcContext) -> !;
}
