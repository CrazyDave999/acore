/// For simplicity, only implement a poll-based stdin.


use super::mmio::UART;
use crate::proc::switch_thread;
pub fn getchar() -> u8 {
    loop {
        if let Some(c) = UART.recv() {
            return c;
        } else {
            switch_thread();
        }
    }
}
