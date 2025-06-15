
    .align 3
    .section .data
    .global _num_app
_num_app:
    .quad 23
    .quad app_0_start
    .quad app_1_start
    .quad app_2_start
    .quad app_3_start
    .quad app_4_start
    .quad app_5_start
    .quad app_6_start
    .quad app_7_start
    .quad app_8_start
    .quad app_9_start
    .quad app_10_start
    .quad app_11_start
    .quad app_12_start
    .quad app_13_start
    .quad app_14_start
    .quad app_15_start
    .quad app_16_start
    .quad app_17_start
    .quad app_18_start
    .quad app_19_start
    .quad app_20_start
    .quad app_21_start
    .quad app_22_start
    .quad app_22_end

    .global _app_names
_app_names:
    .string "adder"
    .string "adder_mutex_blocking"
    .string "cat"
    .string "cd"
    .string "exit"
    .string "forktest"
    .string "forktest2"
    .string "forktest_simple"
    .string "forktree"
    .string "fstat"
    .string "ftest"
    .string "hello"
    .string "init"
    .string "ll"
    .string "ls"
    .string "mkdir"
    .string "shell"
    .string "sig_simple"
    .string "sig_simple2"
    .string "sig_tests"
    .string "sleep"
    .string "threads"
    .string "threads_arg"

    .section .data
    .global app_0_start
    .global app_0_end
    .align 3
app_0_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/adder"
app_0_end:

    .section .data
    .global app_1_start
    .global app_1_end
    .align 3
app_1_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/adder_mutex_blocking"
app_1_end:

    .section .data
    .global app_2_start
    .global app_2_end
    .align 3
app_2_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/cat"
app_2_end:

    .section .data
    .global app_3_start
    .global app_3_end
    .align 3
app_3_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/cd"
app_3_end:

    .section .data
    .global app_4_start
    .global app_4_end
    .align 3
app_4_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/exit"
app_4_end:

    .section .data
    .global app_5_start
    .global app_5_end
    .align 3
app_5_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/forktest"
app_5_end:

    .section .data
    .global app_6_start
    .global app_6_end
    .align 3
app_6_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/forktest2"
app_6_end:

    .section .data
    .global app_7_start
    .global app_7_end
    .align 3
app_7_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/forktest_simple"
app_7_end:

    .section .data
    .global app_8_start
    .global app_8_end
    .align 3
app_8_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/forktree"
app_8_end:

    .section .data
    .global app_9_start
    .global app_9_end
    .align 3
app_9_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/fstat"
app_9_end:

    .section .data
    .global app_10_start
    .global app_10_end
    .align 3
app_10_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/ftest"
app_10_end:

    .section .data
    .global app_11_start
    .global app_11_end
    .align 3
app_11_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/hello"
app_11_end:

    .section .data
    .global app_12_start
    .global app_12_end
    .align 3
app_12_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/init"
app_12_end:

    .section .data
    .global app_13_start
    .global app_13_end
    .align 3
app_13_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/ll"
app_13_end:

    .section .data
    .global app_14_start
    .global app_14_end
    .align 3
app_14_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/ls"
app_14_end:

    .section .data
    .global app_15_start
    .global app_15_end
    .align 3
app_15_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/mkdir"
app_15_end:

    .section .data
    .global app_16_start
    .global app_16_end
    .align 3
app_16_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/shell"
app_16_end:

    .section .data
    .global app_17_start
    .global app_17_end
    .align 3
app_17_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/sig_simple"
app_17_end:

    .section .data
    .global app_18_start
    .global app_18_end
    .align 3
app_18_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/sig_simple2"
app_18_end:

    .section .data
    .global app_19_start
    .global app_19_end
    .align 3
app_19_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/sig_tests"
app_19_end:

    .section .data
    .global app_20_start
    .global app_20_end
    .align 3
app_20_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/sleep"
app_20_end:

    .section .data
    .global app_21_start
    .global app_21_end
    .align 3
app_21_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/threads"
app_21_end:

    .section .data
    .global app_22_start
    .global app_22_end
    .align 3
app_22_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/threads_arg"
app_22_end:
