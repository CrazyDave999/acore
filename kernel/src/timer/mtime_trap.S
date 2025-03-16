# We catch machine mode time interrupt here and delegate it to supervisor mode.
    .section .text.time
    .globl __mtime_trap
    .align 2
__mtime_trap:
	csrrw sp, mscratch, sp
	sd t0, 0(sp)
	sd t1, 1*8(sp)
	sd t2, 2*8(sp)

    ld t0, 3*8(sp) # get the address of mtimercmp
    ld t1, 4*8(sp) # get timer interval
    ld t2, 0(t0) # get current time
    add t2, t2, t1 # next trigger time
    sd t2, 0(t0) # set next trigger time

    li t0, 2
    csrw sip, t0 # raise supervisor software interrupt

	# restore registers
	ld t0, 0(sp)
	ld t1, 1*8(sp)
	ld t2, 2*8(sp)
	csrrw sp, mscratch, sp

	mret
