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

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Display;
use log::{error, info};
use user_lib::console::getchar;
use user_lib::{close, dup, exec, fork, get_abs_path, get_env_var_path, get_exe_path, getcwd, open, waitpid, OpenFlags};

enum State {
    Good,
    Bad,
}

impl Display for State {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            State::Good => write!(f, "^_^"),
            State::Bad => write!(f, "T_T"),
        }
    }
}

fn print_cwd(state: &State, cwd: &str) {
    match state {
        State::Good => {
            info!("{} {}$ ", state, cwd);
        }
        State::Bad => {
            error!("{} {}$ ", state, cwd);
        }
    }
}

#[no_mangle]
pub fn main(_argc: usize, _argv: &[&str]) -> i32 {
    println!("[shell] This is CrazyDave shell.");

    // print the logo, run the logo exe file
    let pid = fork();
    if pid == 0 {
        exec("/bin/logo\0", &[core::ptr::null::<u8>()]);
    } else {
        waitpid(pid as usize, &mut 0);
    }



    let mut line: String = String::new();
    let mut cwd: String = getcwd();
    let mut state = State::Good;

    print_cwd(&state, &cwd);
    loop {
        let c = getchar();
        match c {
            LF | CR => {
                println!("");
                line = line.trim().to_string();
                if !line.is_empty() {
                    let mut args: Vec<String> = line
                        .as_str()
                        .split_whitespace()
                        .map(|arg| {
                            let mut s = String::from(arg);
                            s.push('\0');
                            s
                        })
                        .collect();

                    // redirect input
                    let mut input = String::new();
                    if let Some((idx, _)) = args
                        .iter()
                        .enumerate()
                        .find(|(_, arg)| arg.as_str() == "<\0")
                    {
                        input = args[idx + 1].clone();
                        args.drain(idx..=idx + 1);
                    }

                    // redirect output
                    let mut output = String::new();
                    if let Some((idx, _)) = args
                        .iter()
                        .enumerate()
                        .find(|(_, arg)| arg.as_str() == ">\0")
                    {
                        output = args[idx + 1].clone();
                        args.drain(idx..=idx + 1);
                    }

                    let mut args_addr: Vec<*const u8> =
                        args.iter().map(|arg| arg.as_ptr()).collect();
                    args_addr.push(0 as *const u8); // null-terminate the args

                    let pid = fork();
                    if pid == 0 {
                        // child process
                        if !input.is_empty() {
                            // redirect input
                            let input_fd = open(get_abs_path(&input).as_str(), OpenFlags::RDONLY);
                            if input_fd < 0 {
                                println!("[shell] Error opening input file: '{}'", input);
                                return -4;
                            }
                            close(0);
                            assert_eq!(dup(input_fd as usize), 0);
                            close(input_fd as usize);
                        }

                        if !output.is_empty() {
                            // redirect output
                            let output_fd =
                                open(get_abs_path(&output).as_str(), OpenFlags::CREATE |
                                    OpenFlags::WRONLY);
                            if output_fd < 0 {
                                println!("[shell] Error opening output file: '{}'", output);
                                return -4;
                            }
                            close(1);
                            assert_eq!(dup(output_fd as usize), 1);
                            close(output_fd as usize);
                        }

                        if let Some(path) = get_exe_path(args[0].as_str()) {
                            if exec(&path, args_addr.as_slice()) == -1 {
                                println!("[shell] Error when executing!");
                                return -4;
                            }
                        } else {
                            println!(
                                "[shell] Command '{}' not found. Neither in cwd nor in env var \
                                PATHs: {:?}",
                                args[0],
                                get_env_var_path()
                            );
                            return -5;
                        }

                        unreachable!();
                    } else {
                        let mut exit_code: i32 = 0;
                        let exit_pid = waitpid(pid as usize, &mut exit_code);
                        assert_eq!(pid, exit_pid);
                        // println!("[shell] Process {} exited with code {}", pid, exit_code);
                        if exit_code < 0 {
                            state = State::Bad;
                        } else {
                            state = State::Good;
                        }
                    }
                    line.clear();
                }
                cwd = getcwd();
                print_cwd(&state, &cwd);
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
