pub mod history;

use history::History;
use nix::errno::Errno;
use nix::sys::signal::{signal, SigHandler, Signal};
use nix::sys::wait::{wait, WaitStatus};
use std::cmp::min;
use std::env;
use std::io::{self, stdin, Write};
use std::path::Path;
use std::process::{exit, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

/// some ansi control colors
const COLOR_RED: &str = "\x1B[38;5;9m";
const COLOR_GREEN: &str = "\x1B[38;5;10m";
const COLOR_YELLOW: &str = "\x1B[38;5;11m";
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

    loop {
        // prompt message
        INPUTING.store(true, Ordering::Relaxed);
        print_prompt();

        // read line
        let mut cmd = String::new();
        if let Some(0) = stdin().read_line(&mut cmd).ok() {
            exit(0)
        }
        let mut cmd = cmd.trim().to_string();

        // pre-processing, if ! and !!
        // then find the actuall command from history
        let mut command_changed = false;
        if cmd.starts_with("!") {
            let s = cmd.strip_prefix("!").unwrap().trim();
            cmd = if s.starts_with("!") {
                command_changed = true;
                match history.last().cloned() {
                    Some(cmd) => cmd,
                    _ => continue,
                }
            } else {
                let number = s.parse::<usize>();
                match number {
                    Err(_) => continue,
                    Ok(number) => {
                        command_changed = true;
                        match history.get(number).cloned() {
                            Some(cmd) => cmd,
                            _ => continue,
                        }
                    }
                }
            };
        }
        if command_changed {
            println!("> {}{}{}", COLOR_YELLOW, &cmd, CLEAR_COLOR)
        }
        if cmd != history.last().cloned().unwrap_or_default() {
            history.push(&cmd);
        }

        let tokens: Vec<String> = cmd
            .split_whitespace()
            .map(|s| {
                if s.starts_with("$") {
                    let key = s.strip_prefix("$").unwrap();
                    env::var(key).unwrap_or_default()
                } else {
                    s.to_string()
                }
            })
            .collect();
        let commands: Vec<&[String]> = tokens.split(|s| s == "|").collect();

        // util function to divide program name and args from a command
        let parse_command = |command: &[String]| -> (String, Vec<String>) {
            let mut token_iter = command.iter();
            let prog = token_iter.next().cloned().unwrap_or_default();
            let args = token_iter.map(|s| s.to_owned()).collect();
            (prog, args)
        };

        // execute commands
        INPUTING.store(false, Ordering::Relaxed);
        EXITCODE.store(0, Ordering::Relaxed);
        match commands.len() {
            0 => continue,
            1 => {
                let command = commands[0];
                let (prog, args) = parse_command(command);
                match prog.as_str() {
                    "history" | "cd" | "export" | "exit" => do_built_in(&prog, &args, &mut history),
                    _ => {
                        if let Ok(_) = Command::new(&prog).args(&args).spawn() {
                            while match wait() {
                                Err(Errno::ECHILD) => false,
                                Ok(WaitStatus::Exited(_, code)) => {
                                    EXITCODE.store(code, Ordering::Relaxed);
                                    true
                                }
                                _ => true,
                            } {}
                        }
                    }
                }
            }
            _ => {
                let mut command_iter = commands.iter().peekable();

                // stdin and stdout are changed in loop
                let mut stdin = Stdio::inherit();
                let mut stdout = Stdio::piped();
                while let Some(command) = command_iter.next() {
                    let cur_process_stdout = if command_iter.peek().is_none() {
                        Stdio::inherit()
                    } else {
                        stdout
                    };
                    let (prog, args) = parse_command(command);
                    let mut child = Command::new(&prog)
                        .args(&args)
                        .stdin(stdin)
                        .stdout(cur_process_stdout)
                        .spawn()
                        .expect("fail to execute");
                    if command_iter.peek().is_some() {
                        stdin = Stdio::from(child.stdout.take().expect("failed to open fd"));
                        stdout = Stdio::piped();
                    } else {
                        // only for rust analyzer... in fact only the last one goes here :)
                        break;
                    }
                }
                while match wait() {
                    Ok(_) => true,
                    _ => false,
                } {}
            }
        }
    }
}

/// built-in commands
fn do_built_in(prog: &String, args: &Vec<String>, history: &mut History) -> () {
    match prog.as_str() {
        "history" => {
            let number = args.get(0).cloned().unwrap_or_default();
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
            let dir = args.get(0).cloned();
            let home = env::var("HOME");
            let dir = match dir {
                Some(dir) if dir == "~" || dir.starts_with("~/") => match home {
                    Ok(home) => home + dir.strip_prefix("~").unwrap(),
                    _ => {
                        println!("$HOME is unset");
                        String::new()
                    }
                },
                Some(dir) => dir,
                _ => match home {
                    Ok(home) => home,
                    _ => {
                        println!("$HOME is unset");
                        String::new()
                    }
                },
            };
            if let Err(_) = env::set_current_dir(dir) {
                println!("Changing current dir failed")
            }
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
        _ => (),
    }
}

fn print_prompt() -> () {
    let exit_code = EXITCODE.load(Ordering::Relaxed);
    if exit_code != 0 {
        print!("{}[{}]{}", COLOR_RED, exit_code, CLEAR_COLOR);
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
