pub const VIRT_TEST: usize = 0x10_0000;

pub const FINISHER_PASS: usize = 0x5555;

pub const KERNEL_HEAP_SIZE: usize = 0x30_0000;

pub const PAGE_SIZE: usize = 0x1000;
pub const PAGE_SIZE_BITS: usize = 0xc;
pub const PA_WIDTH_SV39: usize = 56;
pub const PPN_WIDTH_SV39: usize = PA_WIDTH_SV39 - PAGE_SIZE_BITS;

pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;

pub const MTIME: usize = 0x0200_bff8;
pub const MTIMECMP: usize = 0x0200_4000;

pub const MEMORY_END: usize = 0x8080_0000;
pub const USER_STACK_SIZE: usize = 4096 * 2;
