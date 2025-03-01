use core::ptr::{read_volatile, write_volatile};

use bitflags::bitflags;
use lazy_static::lazy_static;

macro_rules! wait_for {
    ($cond:expr) => {
        while !$cond {
            core::hint::spin_loop();
        }
    };
}

bitflags! {
    struct Ier: u8{
        const EN = 1 << 0;
    }
    struct Fcr: u8 {
        const EN = 1 << 0;
        const CLR_RX = 1 << 1;
        const CLR_TX = 1 << 2;
    }
    struct Lsr: u8 {
        const DA = 1 << 0; // Data available
        const TE = 1 << 5; // THR is empty
    }
    struct Lcr: u8 {
        const DATA_8 = 0b11; // 8-bit data
        const DLAB = 1 << 7; // Divisor latch access bit
    }
    struct Mcr:u8 {
        const DTR = 1 << 0;
    }
}

pub struct Uart(usize);

impl Uart {
    pub fn new(base: usize) -> Self {
        Uart(base)
    }
    fn read_rbr(&self) -> u8 {
        unsafe { read_volatile((self.0 + 0) as *const u8) }
    }
    fn write_thr(&self, c: u8) {
        unsafe { write_volatile((self.0 + 0) as *mut u8, c) }
    }
    fn read_ier(&self) -> u8 {
        unsafe { read_volatile((self.0 + 1) as *const u8) }
    }
    fn write_ier(&self, c: u8) {
        unsafe { write_volatile((self.0 + 1) as *mut u8, c) }
    }
    fn read_iir(&self) -> u8 {
        unsafe { read_volatile((self.0 + 2) as *const u8) }
    }
    fn write_fcr(&self, c: u8) {
        unsafe { write_volatile((self.0 + 2) as *mut u8, c) }
    }
    fn read_lcr(&self) -> u8 {
        unsafe { read_volatile((self.0 + 3) as *const u8) }
    }
    fn write_lcr(&self, c: u8) {
        unsafe { write_volatile((self.0 + 3) as *mut u8, c) }
    }
    fn read_mcr(&self) -> u8 {
        unsafe { read_volatile((self.0 + 4) as *const u8) }
    }
    fn write_mcr(&self, c: u8) {
        unsafe { write_volatile((self.0 + 4) as *mut u8, c) }
    }

    fn read_lsr(&self) -> u8 {
        unsafe { read_volatile((self.0 + 5) as *const u8) }
    }
    fn read_msr(&self) -> u8 {
        unsafe { read_volatile((self.0 + 6) as *const u8) }
    }

    pub fn init(&self) {
        self.write_ier(0);
        self.write_lcr(Lcr::DLAB.bits());
        self.write_thr(0x03);
        self.write_ier(0);
        self.write_lcr(Lcr::DATA_8.bits());
        self.write_fcr(0);
        self.write_mcr(Mcr::DTR.bits());
        self.write_ier(Ier::EN.bits() );
    }

    pub fn send(&self, c: u8) {
        wait_for!(self.read_lsr() & Lsr::TE.bits() != 0);
        self.write_thr(c);
    }
    pub fn recv(&self) -> Option<u8> {
        if self.read_lsr() & Lsr::DA.bits() != 0 {
            Some(self.read_rbr())
        } else {
            None
        }
    }
}

lazy_static! {
    pub static ref UART: Uart = Uart::new(0x1000_0000);
}
