use crate::println;
use crate::proc::{get_cur_proc, get_cur_thread, switch_thread};
use bitflags::*;

pub const MAX_SIG: usize = 31;

bitflags! {
    pub struct SignalFlags: u32 {
        const SIGDEF = 1; // Default signal handling
        const SIGHUP = 1 << 1;
        const SIGINT = 1 << 2;
        const SIGQUIT = 1 << 3;
        const SIGILL = 1 << 4;
        const SIGTRAP = 1 << 5;
        const SIGABRT = 1 << 6;
        const SIGBUS = 1 << 7;
        const SIGFPE = 1 << 8;
        const SIGKILL = 1 << 9;
        const SIGUSR1 = 1 << 10;
        const SIGSEGV = 1 << 11;
        const SIGUSR2 = 1 << 12;
        const SIGPIPE = 1 << 13;
        const SIGALRM = 1 << 14;
        const SIGTERM = 1 << 15;
        const SIGSTKFLT = 1 << 16;
        const SIGCHLD = 1 << 17;
        const SIGCONT = 1 << 18;
        const SIGSTOP = 1 << 19;
        const SIGTSTP = 1 << 20;
        const SIGTTIN = 1 << 21;
        const SIGTTOU = 1 << 22;
        const SIGURG = 1 << 23;
        const SIGXCPU = 1 << 24;
        const SIGXFSZ = 1 << 25;
        const SIGVTALRM = 1 << 26;
        const SIGPROF = 1 << 27;
        const SIGWINCH = 1 << 28;
        const SIGIO = 1 << 29;
        const SIGPWR = 1 << 30;
        const SIGSYS = 1 << 31;
    }
}

impl SignalFlags {
    pub fn check_error(&self) -> Option<(i32, &'static str)> {
        if self.contains(Self::SIGINT) {
            Some((-2, "Killed, SIGINT=2"))
        } else if self.contains(Self::SIGILL) {
            Some((-4, "Illegal Instruction, SIGILL=4"))
        } else if self.contains(Self::SIGABRT) {
            Some((-6, "Aborted, SIGABRT=6"))
        } else if self.contains(Self::SIGFPE) {
            Some((-8, "Erroneous Arithmetic Operation, SIGFPE=8"))
        } else if self.contains(Self::SIGKILL) {
            Some((-9, "Killed, SIGKILL=9"))
        } else if self.contains(Self::SIGSEGV) {
            Some((-11, "Segmentation Fault, SIGSEGV=11"))
        } else {
            //println!("[K] signalflags check_error  {:?}", self);
            None
        }
    }
}

pub fn check_signals_error_of_current() -> Option<(i32, &'static str)> {
    let cur_proc = get_cur_proc();
    let inner = cur_proc.exclusive_access();
    // println!(
    //     "[K] check_signals_error_of_current {:?}",
    //     inner.signals
    // );
    inner.signals.check_error()
}
pub fn current_add_signal(signal: SignalFlags) {
    let cur_proc = get_cur_proc();
    let mut inner = cur_proc.exclusive_access();
    inner.signals |= signal;
    // println!(
    //     "[K] current_add_signal:: current task sigflag {:?}",
    //     inner.signals
    // );
}
fn call_kernel_signal_handler(signal: SignalFlags) {
    let cur_proc = get_cur_proc();
    let mut inner = cur_proc.exclusive_access();

    match signal {
        SignalFlags::SIGSTOP => {
            inner.frozen = true;
            inner.signals ^= SignalFlags::SIGSTOP;
        }
        SignalFlags::SIGCONT => {
            if inner.signals.contains(SignalFlags::SIGCONT) {
                inner.signals ^= SignalFlags::SIGCONT;
                inner.frozen = false;
            }
        }
        _ => {
            inner.killed = true;
        }
    }
}

fn call_user_signal_handler(sig: usize, signal: SignalFlags) {
    let cur_thr = get_cur_thread().unwrap();
    let cur_thr_inner = cur_thr.exclusive_access();
    let cur_proc = cur_thr.pcb.upgrade().unwrap();
    let mut cur_proc_inner = cur_proc.exclusive_access();

    let handler = cur_proc_inner.signal_actions.table[sig].handler;
    if handler != 0 {
        // user handler

        // handle flag
        cur_proc_inner.handling_sig = sig as isize;
        cur_proc_inner.signals ^= signal;

        // modify trap ctx to jump to user handler
        // backup trap ctx
        let trap_ctx = cur_thr_inner.get_trap_ctx();
        cur_proc_inner.trap_ctx_backup = Some(*trap_ctx);

        // modify trapframe
        trap_ctx.sepc = handler;

        // put args (a0)
        trap_ctx.x[10] = sig;
    } else {
        // default action
        println!("[K] task/call_user_signal_handler: default action: ignore it or kill process");
    }
}

/// Check signals received by current proc and handle them.
fn check_pending_signals() {
    for sig in 0..(MAX_SIG + 1) {
        let cur_proc = get_cur_proc();
        let inner = cur_proc.exclusive_access();
        let signal = SignalFlags::from_bits(1 << sig).unwrap();
        if inner.signals.contains(signal) && (!inner.signal_mask.contains(signal)) {
            let mut masked = true;
            let handling_sig = inner.handling_sig;
            if handling_sig == -1 {
                masked = false;
            } else if !inner.signal_actions.table[handling_sig as usize]
                .mask
                .contains(signal)
            {
                masked = false;
            }
            if !masked {
                drop(inner);
                drop(cur_proc);
                if signal == SignalFlags::SIGKILL
                    || signal == SignalFlags::SIGSTOP
                    || signal == SignalFlags::SIGCONT
                    || signal == SignalFlags::SIGDEF
                {
                    // signal is a kernel signal
                    call_kernel_signal_handler(signal);
                } else {
                    // signal is a user signal
                    call_user_signal_handler(sig, signal);
                    return;
                }
            }
        }
    }
}
pub fn handle_signals() {
    loop {
        check_pending_signals();
        let (frozen, killed) = {
            let cur_proc = get_cur_proc();
            let inner = cur_proc.exclusive_access();
            (inner.frozen, inner.killed)
        };
        if !frozen || killed {
            break;
        }
        switch_thread();
    }
}
