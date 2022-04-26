pub mod history;

use history::History;
use nix::errno::Errno;
use nix::sys::signal::{signal, SigHandler, Signal};
use nix::sys::wait::{wait, WaitStatus};
use std::cmp::min;
use std::io::{stdin, Write};
use std::path::Path;
use std::process::{exit, Child, Command};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::{env, io};

const COLOR_RED: &str = "\x1B[38;5;9m";
const COLOR_GREEN: &str = "\x1B[38;5;10m";
const CLEAR_COLOR: &str = "\x1B[0m";

/// indicates last task exit code
static EXITCODE: AtomicI32 = AtomicI32::new(0);

/// INPUTING indicates whether the shell is waiting for user input
static INPUTING: AtomicBool = AtomicBool::new(true);
extern "C" fn handle_sigint(_: libc::c_int) {
    println!();
    if INPUTING.load(Ordering::Relaxed) {
        print_prompt()
    }
}

fn main() -> ! {
    unsafe { signal(Signal::SIGINT, SigHandler::Handler(handle_sigint)) }
        .expect("Error changing SIGINT handler");

    let home = env::var("HOME");
    let history_file_name = match home {
        Ok(home) => home + "/.llysh_history",
        _ => "/tmp/.llysh_history".to_string(),
    };
    let mut history = History::new(history_file_name).expect("History file i/o error");
    let mut prev_cmd = String::new();

    loop {
        // prompt message
        INPUTING.store(true, Ordering::Relaxed);
        print_prompt();

        let mut cmd = String::new();
        match stdin().read_line(&mut cmd) {
            Ok(0) => exit(0),
            _ => (),
        }

        // if ! and !!
        let mut command_changed = false;
        let get_command = |command: Option<&String>| -> String {
            match command {
                Some(s) => s.clone(),
                _ => String::new(),
            }
        };
        if cmd.starts_with("!") {
            let s = cmd.strip_prefix("!").unwrap().trim();
            if s.starts_with("!") {
                command_changed = true;
                cmd = prev_cmd.clone();
            } else {
                let number = s.parse::<usize>();
                match number {
                    Ok(0) | Err(_) => {
                        println!("Invalid input number");
                        cmd = String::new()
                    }
                    Ok(number) => {
                        command_changed = true;
                        cmd = get_command(history.get(number));
                    }
                }
            }
        }
        if command_changed {
            println!("{}", &cmd)
        } else {
            history.push(&cmd);
        }
        prev_cmd = cmd.clone();

        let args = cmd.split_whitespace();
        let mut args = args.map(|s| {
            if s.starts_with("$") {
                let key = s.strip_prefix("$").unwrap();
                match env::var(key) {
                    Ok(value) => value,
                    _ => String::new(),
                }
            } else {
                s.to_string()
            }
        });
        let prog = args.next();

        INPUTING.store(false, Ordering::Relaxed);
        EXITCODE.store(0, Ordering::Relaxed);
        match prog {
            None => (),
            Some(prog) => match prog.as_str() {
                "history" => {
                    let number = args.next();
                    let number = match number {
                        Some(number) => number,
                        _ => "10".to_string(), // default history nums
                    };
                    let number = match number.parse::<usize>() {
                        Ok(number) => number,
                        _ => {
                            println!("Invalid input. Show 10 results...");
                            10
                        }
                    };
                    let history_size = history.size();
                    for i in (0..min(number, history_size)).rev() {
                        println!("{:5}  {}", history_size - i, history.rget(i).unwrap())
                    }
                }
                "cd" => {
                    let dir = args.next();
                    let dir = match dir {
                        Some(dir) => dir,
                        _ => match env::var("HOME") {
                            Ok(home) => home,
                            _ => {
                                println!("$HOME is unset");
                                String::new()
                            }
                        },
                    };
                    match env::set_current_dir(dir) {
                        Err(_) => println!("Changing current dir failed"),
                        _ => (),
                    }
                }
                "pwd" => {
                    let err = "Getting current dir failed";
                    println!("{}", env::current_dir().expect(err).to_str().expect(err));
                }
                "export" => {
                    for arg in args {
                        let mut assign = arg.split("=");
                        let name = assign.next().expect("No variable name");
                        let value = assign.next().expect("No variable value");
                        env::set_var(name, value);
                    }
                }
                "exit" => {
                    exit(0);
                }
                _ => match subprocess(&prog, &args.map(|s| s.to_string()).collect()) {
                    Some(_) => loop {
                        match wait() {
                            Err(Errno::ECHILD) => break,
                            Ok(WaitStatus::Exited(_, code)) => {
                                EXITCODE.store(code, Ordering::Relaxed)
                            }
                            _ => (),
                        }
                    },
                    _ => println!("Failed to start the program: {}", &prog),
                },
            },
        }
    }
}

/// returns Some(Child) if successful
fn subprocess(target: &String, args: &Vec<String>) -> Option<Child> {
    let mut command = Command::new(target);
    let command = command.args(args);
    let command = command.spawn().ok()?;
    Some(command)
}

fn print_prompt() -> () {
    let exit_code = EXITCODE.load(Ordering::Relaxed);
    if exit_code != 0 {
        print!("{}[{}]{}> ", COLOR_RED, exit_code, CLEAR_COLOR);
    }
    print!("{}{}{}> ", COLOR_GREEN, &prompt_path(), CLEAR_COLOR);
    io::stdout().flush().expect("error printing prompt");
}

fn prompt_path() -> String {
    let cwd = env::current_dir().expect("Getting current dir failed");
    let cwd = cwd.as_path();
    let home = env::var("HOME");
    let path_err = "Invalid path name";
    match home {
        Ok(home) => {
            if cwd == Path::new(&home) {
                "~".to_string()
            } else if cwd.starts_with(&home) {
                "~/".to_string()
                    + cwd
                        .strip_prefix(&home)
                        .expect(path_err)
                        .to_str()
                        .expect(path_err)
            } else {
                cwd.to_str().expect(path_err).to_string()
            }
        }
        _ => cwd.to_str().expect(path_err).to_string(),
    }
}
