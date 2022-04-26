use nix::errno::Errno;
use nix::sys::signal::{signal, SigHandler, Signal};
use nix::sys::wait::wait;
use std::io::{stdin, Write};
use std::path::Path;
use std::process::{exit, Child, Command};
use std::{env, io};

const COLOR_GREEN: &str = "\x1B[38;5;10m";
const CLEAR_COLOR: &str = "\x1B[0m";

/// INPUTING indicates whether the shell is waiting for user input
use std::sync::atomic::{AtomicBool, Ordering};
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

    loop {
        // prompt message
        INPUTING.store(true, Ordering::Relaxed);
        print_prompt();

        let mut cmd = String::new();
        match stdin().read_line(&mut cmd) {
            Ok(0) => exit(0),
            _ => (),
        }
        let args = cmd.split_whitespace();
        let mut args = args.map(|s| {
            if s.starts_with("$") {
                let key = s.strip_prefix("$").unwrap();
                match env::var(key) {
                    Ok(value) => value,
                    _ => "".to_string(),
                }
            } else {
                s.to_string()
            }
        });
        let prog = args.next();

        INPUTING.store(false, Ordering::Relaxed);
        match prog {
            None => (),
            Some(prog) => match prog.as_str() {
                "cd" => {
                    let dir = args.next();
                    let dir = match dir {
                        Some(dir) => dir,
                        _ => match env::var("HOME") {
                            Ok(home) => home,
                            _ => {
                                println!("$HOME is unset");
                                "".to_string()
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
