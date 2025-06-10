#![no_std]
#![no_main]
#![allow(clippy::println_empty_string)]

extern crate alloc;

#[macro_use]
extern crate user_lib;

const LF: u8 = 0x0au8;
const CR: u8 = 0x0du8;
const DL: u8 = 0x7fu8;
const BS: u8 = 0x08u8;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Display;
use log::{error, info};
use user_lib::console::getchar;
use user_lib::{exec, fork, waitpid};

enum State {
    Good,
    Bad,
}

struct Path {
    path: Vec<String>,
}

impl Path {
    pub fn new() -> Self {
        Path { path: Vec::new() }
    }
    pub fn push(&mut self, dir: String) {
        self.path.push(dir);
    }
    pub fn pop(&mut self) -> Option<String> {
        self.path.pop()
    }
}
impl Display for Path {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut result = String::from("/");
        for s in self.path.iter() {
            result.push_str(s);
            result.push('/');
        }
        write!(f, "{}", result)
    }
}

impl Display for State {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            State::Good => write!(f, "^_^"),
            State::Bad => write!(f, "T_T"),
        }
    }
}

pub fn print_pwd(state: &State, pwd: &Path) {
    match state {
        State::Good => {
            info!("{} {}$ ", state, pwd);
        }
        State::Bad => {
            error!("{} {}$ ", state, pwd);
        }
    }
}

#[no_mangle]
pub fn main() -> i32 {
    println!("[shell] This is CrazyDave shell.");
    let mut line: String = String::new();
    let pwd = Path::new();
    let mut state = State::Good;

    print_pwd(&state, &pwd);
    loop {
        let c = getchar();
        match c {
            LF | CR => {
                println!("");
                if !line.is_empty() {
                    // file name
                    line.push('\0');

                    let pid = fork();
                    if pid == 0 {
                        // child process
                        let abs_path = if line.starts_with('/') {
                            line
                        } else {
                            format!("{}{}", pwd, line)
                        };
                        if exec(abs_path.as_str()) == -1 {
                            println!("[shell] Error when executing!");
                            return -4;
                        }
                        unreachable!();
                    } else {
                        let mut exit_code: i32 = 0;
                        let exit_pid = waitpid(pid as usize, &mut exit_code);
                        assert_eq!(pid, exit_pid);
                        println!("[shell] Process {} exited with code {}", pid, exit_code);
                        if exit_code < 0 {
                            state = State::Bad;
                        } else {
                            state = State::Good;
                        }
                    }
                    line.clear();
                }
                print_pwd(&state, &pwd);
            }
            BS | DL => {
                if !line.is_empty() {
                    print!("{}", BS as char);
                    print!(" ");
                    print!("{}", BS as char);
                    line.pop();
                }
            }
            _ => {
                print!("{}", c as char);
                line.push(c as char);
            }
        }
    }
}
