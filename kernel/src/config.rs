pub const VIRT_TEST: usize = 0x10_0000;

pub const FINISHER_PASS: usize = 0x5555;

pub const KERNEL_HEAP_SIZE: usize = 0x30_0000;

pub const PAGE_SIZE: usize = 0x1000;
pub const PAGE_SIZE_BITS: usize = 0xc;
pub const PA_WIDTH_SV39: usize = 56;
pub const VA_WIDTH_SV39: usize = 39;
pub const PPN_WIDTH_SV39: usize = PA_WIDTH_SV39 - PAGE_SIZE_BITS;
pub const VPN_WIDTH_SV39: usize = VA_WIDTH_SV39 - PAGE_SIZE_BITS;

pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1; // 0xffff_ffff_ffff_f000
/// The base address of the trap ctx of thread 0 in each process.
pub const TRAP_CONTEXT_BASE: usize = TRAMPOLINE - PAGE_SIZE; // 0xffff_ffff_ffff_e000
// 0x80018a24
pub const VIRT_CLINT: usize = 0x0200_0000;
pub const VIRT_CLINT_SIZE: usize = 0x10000;

pub const VIRT_UART0: usize = 0x1000_0000;
pub const VIRT_UART0_SIZE: usize = 0x100;
pub const VIRT_UART_VIRTIO: usize = 0x1000_1000;
pub const VIRT_UART_VIRTIO_SIZE: usize = 0x1000;

pub const MTIME: usize = 0x0200_bff8;
pub const MTIMECMP: usize = 0x0200_4000;

pub const MEMORY_END: usize = 0x8800_0000;
pub const USER_STACK_SIZE: usize = 4096 * 4;
pub const KERNEL_STACK_SIZE: usize = 4096 * 4;
