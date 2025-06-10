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
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Display;
use log::{error, info};
use user_lib::console::getchar;
use user_lib::{close, dup, exec, fork, open, waitpid, OpenFlags};

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

fn print_pwd(state: &State, pwd: &Path) {
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
pub fn main(_argc: usize, _argv: &[&str]) -> i32 {
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
                    let args: Vec<&str> = line.as_str().split(' ').collect();
                    let mut args_copy: Vec<String> = args
                        .iter()
                        .enumerate()
                        .map(|(i, &arg)| {
                            if i == 0 && !arg.starts_with('/') {
                                // modify relative path to absolute path
                                format!("{}{}\0", pwd, arg)
                            } else {
                                format!("{}\0", arg)
                            }
                        })
                        .collect();

                    // redirect input
                    let mut input = String::new();
                    if let Some((idx, _)) = args_copy
                        .iter()
                        .enumerate()
                        .find(|(_, arg)| arg.as_str() == "<\0")
                    {
                        input = args_copy[idx + 1].clone();
                        args_copy.drain(idx..=idx + 1);
                    }

                    // redirect output
                    let mut output = String::new();
                    if let Some((idx, _)) = args_copy
                        .iter()
                        .enumerate()
                        .find(|(_, arg)| arg.as_str() == ">\0")
                    {
                        output = args_copy[idx + 1].clone();
                        args_copy.drain(idx..=idx + 1);
                    }

                    let mut args_addr: Vec<*const u8> =
                        args_copy.iter().map(|arg| arg.as_ptr()).collect();
                    args_addr.push(0 as *const u8); // null-terminate the args

                    let pid = fork();
                    if pid == 0 {
                        // child process

                        if !input.is_empty() {
                            // redirect input
                            let input_fd = open(input.as_str(), OpenFlags::RDONLY);
                            if input_fd < 0 {
                                println!("[shell] Error opening input file: {}", input);
                                return -4;
                            }
                            close(0);
                            assert_eq!(dup(input_fd as usize), 0);
                            close(input_fd as usize);
                        }

                        if !output.is_empty() {
                            // redirect output
                            let output_fd =
                                open(output.as_str(), OpenFlags::CREATE | OpenFlags::WRONLY);
                            if output_fd < 0 {
                                println!("[shell] Error opening output file: {}", output);
                                return -4;
                            }
                            close(1);
                            assert_eq!(dup(output_fd as usize), 1);
                            close(output_fd as usize);
                        }

                        if exec(args_copy[0].as_str(), args_addr.as_slice()) == -1 {
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
