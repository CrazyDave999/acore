use super::File;
use crate::sync::UPSafeCell;
use alloc::sync::{Arc, Weak};
use crate::proc::switch_thread;

pub struct Pipe {
    readable: bool,
    writable: bool,
    buffer: Arc<UPSafeCell<PipeRingBuffer>>,
}

impl Pipe {
    pub fn read_end_with_buffer(buffer: Arc<UPSafeCell<PipeRingBuffer>>) -> Self {
        Self {
            readable: true,
            writable: false,
            buffer,
        }
    }
    pub fn write_end_with_buffer(buffer: Arc<UPSafeCell<PipeRingBuffer>>) -> Self {
        Self {
            readable: false,
            writable: true,
            buffer,
        }
    }
}

const RING_BUFFER_SIZE: usize = 32;

#[derive(Copy, Clone, PartialEq)]
enum RingBufferStatus {
    Full,
    Empty,
    Normal,
}

pub struct PipeRingBuffer {
    arr: [u8; RING_BUFFER_SIZE],
    head: usize,
    tail: usize,
    status: RingBufferStatus,
    write_end: Option<Weak<Pipe>>,
}

impl PipeRingBuffer {
    pub fn new() -> Self {
        Self {
            arr: [0; RING_BUFFER_SIZE],
            head: 0,
            tail: 0,
            status: RingBufferStatus::Empty,
            write_end: None,
        }
    }
    pub fn set_write_end(&mut self, write_end: &Arc<Pipe>) {
        self.write_end = Some(Arc::downgrade(write_end));
    }
    pub fn write_byte(&mut self, byte: u8) {
        self.status = RingBufferStatus::Normal;
        self.arr[self.tail] = byte;
        self.tail = (self.tail + 1) % RING_BUFFER_SIZE;
        if self.tail == self.head {
            self.status = RingBufferStatus::Full;
        }
    }
    pub fn read_byte(&mut self) -> u8 {
        self.status = RingBufferStatus::Normal;
        let c = self.arr[self.head];
        self.head = (self.head + 1) % RING_BUFFER_SIZE;
        if self.head == self.tail {
            self.status = RingBufferStatus::Empty;
        }
        c
    }
    pub fn available_read(&self) -> usize {
        if self.status == RingBufferStatus::Empty {
            0
        } else if self.tail > self.head {
            self.tail - self.head
        } else {
            self.tail + RING_BUFFER_SIZE - self.head
        }
    }
    pub fn available_write(&self) -> usize {
        if self.status == RingBufferStatus::Full {
            0
        } else {
            RING_BUFFER_SIZE - self.available_read()
        }
    }
    pub fn all_write_ends_closed(&self) -> bool {
        self.write_end.as_ref().unwrap().upgrade().is_none()
    }
}

/// Creates a pair of pipes (read end, write end).
pub fn make_pipe_pair() -> (Arc<Pipe>, Arc<Pipe>) {
    let buffer = Arc::new(unsafe { UPSafeCell::new(PipeRingBuffer::new()) });
    let read_end = Arc::new(Pipe::read_end_with_buffer(buffer.clone()));
    let write_end = Arc::new(Pipe::write_end_with_buffer(buffer.clone()));
    buffer.exclusive_access().set_write_end(&write_end);
    (read_end, write_end)
}

impl File for Pipe {
    fn readable(&self) -> bool {
        self.readable
    }

    fn writable(&self) -> bool {
        self.writable
    }
    fn read(&self, buf: &mut [u8]) -> usize {
        assert!(self.readable());
        let len = buf.len();
        let mut read_cnt = 0usize;
        loop {
            let mut ring_buffer = self.buffer.exclusive_access();
            let cur_read_cnt = ring_buffer.available_read();
            if cur_read_cnt == 0 {
                if ring_buffer.all_write_ends_closed() {
                    return read_cnt;
                }
                drop(ring_buffer);
                switch_thread();
                continue;
            }
            if read_cnt == len {
                return read_cnt;
            }
            for _ in 0..cur_read_cnt {
                buf[read_cnt] = ring_buffer.read_byte();
                read_cnt += 1;
                if read_cnt == len {
                    return read_cnt;
                }
            }
        }
    }
    fn write(&self, buf: &[u8]) -> usize {
        assert!(self.writable());
        let len = buf.len();
        let mut write_cnt = 0usize;
        loop {
            let mut ring_buffer = self.buffer.exclusive_access();
            let cur_write_cnt = ring_buffer.available_write();
            if cur_write_cnt == 0 {
                drop(ring_buffer);
                switch_thread();
                continue;
            }
            if write_cnt == len {
                return write_cnt;
            }
            for _ in 0..cur_write_cnt {
                ring_buffer.write_byte(buf[write_cnt]);
                write_cnt += 1;
                if write_cnt == len {
                    return write_cnt;
                }
            }
        }
    }
    fn seek(&self, _: usize) -> usize {
        panic!("Pipe does not support seek operation");
    }
}