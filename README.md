# ACORE implementation

This is a simple implementation of an RV64 operating system kernel, written in Rust and asm. And it is my homework 
for the Operating System course in SJTU.

```
          _____                    _____                   _______                   _____                    _____
         /\    \                  /\    \                 /::\    \                 /\    \                  /\    \
        /::\    \                /::\    \               /::::\    \               /::\    \                /::\    \
       /::::\    \              /::::\    \             /::::::\    \             /::::\    \              /::::\    \
      /::::::\    \            /::::::\    \           /::::::::\    \           /::::::\    \            /::::::\    \
     /:::/\:::\    \          /:::/\:::\    \         /:::/~~\:::\    \         /:::/\:::\    \          /:::/\:::\    \
    /:::/__\:::\    \        /:::/  \:::\    \       /:::/    \:::\    \       /:::/__\:::\    \        /:::/__\:::\    \
   /::::\   \:::\    \      /:::/    \:::\    \     /:::/    / \:::\    \     /::::\   \:::\    \      /::::\   \:::\    \
  /::::::\   \:::\    \    /:::/    / \:::\    \   /:::/____/   \:::\____\   /::::::\   \:::\    \    /::::::\   \:::\    \
 /:::/\:::\   \:::\    \  /:::/    /   \:::\    \ |:::|    |     |:::|    | /:::/\:::\   \:::\____\  /:::/\:::\   \:::\    \
/:::/  \:::\   \:::\____\/:::/____/     \:::\____\|:::|____|     |:::|    |/:::/  \:::\   \:::|    |/:::/__\:::\   \:::\____\
\::/    \:::\  /:::/    /\:::\    \      \::/    / \:::\    \   /:::/    / \::/   |::::\  /:::|____|\:::\   \:::\   \::/    /
 \/____/ \:::\/:::/    /  \:::\    \      \/____/   \:::\    \ /:::/    /   \/____|:::::\/:::/    /  \:::\   \:::\   \/____/
          \::::::/    /    \:::\    \                \:::\    /:::/    /          |:::::::::/    /    \:::\   \:::\    \
           \::::/    /      \:::\    \                \:::\__/:::/    /           |::|\::::/    /      \:::\   \:::\____\
           /:::/    /        \:::\    \                \::::::::/    /            |::| \::/____/        \:::\   \::/    /
          /:::/    /          \:::\    \                \::::::/    /             |::|  ~|               \:::\   \/____/
         /:::/    /            \:::\    \                \::::/    /              |::|   |                \:::\    \
        /:::/    /              \:::\____\                \::/____/               \::|   |                 \:::\____\
        \::/    /                \::/    /                 ~~                      \:|   |                  \::/    /
         \/____/                  \/____/                                           \|___|                   \/____/


```

## Usage
```bash
cd kernel
make build
make run
```
The `make build` command will compile the kernel, user lib and pack the into `fs.img`, with the `AcoreFileSystem` 
implemented in this repository. If build correctly, you will see something like:
```
 ________      ________      ________      ________      _______           ________  ________
|\   __  \    |\   ____\    |\   __  \    |\   __  \    |\  ___ \         |\  _____\|\   ____\
\ \  \|\  \   \ \  \___|    \ \  \|\  \   \ \  \|\  \   \ \   __/|        \ \  \__/ \ \  \___|_
 \ \   __  \   \ \  \        \ \  \\\  \   \ \   _  _\   \ \  \_|/__       \ \   __\ \ \_____  \
  \ \  \ \  \   \ \  \____    \ \  \\\  \   \ \  \\  \|   \ \  \_|\ \       \ \  \_|  \|____|\  \
   \ \__\ \__\   \ \_______\   \ \_______\   \ \__\\ _\    \ \_______\       \ \__\     ____\_\  \
    \|__|\|__|    \|_______|    \|_______|    \|__|\|__|    \|_______|        \|__|    |\_________\
                                                                                       \|_________|

AcoreFS packer started...
src_path = ../user/src/bin/
target_path = ../user/target/riscv64gc-unknown-none-elf/release/
Successfully created fs.img with size: 16777216 bytes
Found 61 apps to pack
Successfully created AcoreFileSystem
Successfully created root inode
Processing file: fantastic_text, size: 128336 bytes
...
```

The `make run` command will launch the kernel.

## Implementation Details

I implemented following OS core components, with some parts depend on the rCore project.

### Bootloader
- Initialization(Rust SBI for qemu UART)
- Entering M mode for the kernel

After init progress, we jump from M mode to S mode, and the kernel take control.

### Allocator
- Buddy allocator for kernel heap memory management
- Frame allocator for virtual memory management

### Page table

I implemented a SV39 page table. Also I created a higher level abstraction for virtual memory management, i.e., 
`MemoryManager` struct. 

### Console

I implemented stdio methods and printing macros, which can be used in both kernel and user space.

### Message & Data transfer

- User -> Kernel
  - Syscall
  - MemoryManager read method
- Kernel -> User
  - Syscall return value
  - MemoryManager write method
- Kernel -> Kernel
  - Direct function call
- User -> User
  - IPC methods (signals, pipes, etc.)

### Process & Thread

Process implementation is mainly for isolation (e.g. addr space and mutexes), while thread is for concurrency, 
sharing addr space but with independent trap context.

- Process Loading (from disk with the help of AcoreFS)
  - ELF parsing
  - Sections loading (ref to page table)
- PCB design
  - Pid & State
  - Children and parent pointers
  - Memory manager
  - File descriptor table
  - Threads
  - Mutexes & Condvars
- TCB design
  - Tid & State
  - Unified resources
  - Trap context
  - Thread context
- Syscall
  - For process: fork, exec, exit, wait, yield, getpid, signal actions, etc.
  - For thread: create, gettid, waittid, etc.
  - For syncronization: mutex lock/unlock, condvar wait/signal, etc.
  - For file system: open, read, write, close, pipe, and many supporting syscalls for shell implementation, like 
    fstat, getcwd, cd, mv, cp, rm, etc.
- Thread manager
  - Multi-threading support
  - Thread creation, destruction, and management
  - Unified thread resources management
  - Thread interaction (e.g. IPC methods)
- Scheduler
  - Trap context switching and restoring, at the granularity of threads
  - Scheduling algorithm: naive FCFS
  - Timer interrupt
  - Blocking and waiting mechanism

### Synchronization primitives
- Mutex(both Spin and Blocked)
- Condition variables

### File system
- Inode design: supporting any degree of indirect blocks
- Multi-level directory structure
- File/Directory creation, deletion
- File/Directory reading, writing
- File/Directory listing, finding, renaming, moving

### Others
- User lib with syscall wrappers supporting functions
- Shell implementation
  - Command parsing
  - Redirection for input/output, pipes
  - Environment variable PATH
  - Built-in commands: cat, ls, ll, fstat, cd, touch, mkdir, cp, mv, rm, echo, etc.
- Some DIY features
- Tests for major components