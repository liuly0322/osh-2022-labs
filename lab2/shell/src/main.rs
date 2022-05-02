pub mod history;

use history::History;
use nix::sys::signal::{signal, SigHandler, Signal};
use nix::sys::wait::wait;
use std::cmp::min;
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, stdin, Write};
use std::path::Path;
use std::process::{exit, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

const COLOR_GREEN: &str = "\x1B[38;5;10m";
const COLOR_YELLOW: &str = "\x1B[38;5;11m";
const CLEAR_COLOR: &str = "\x1B[0m";

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
        // then replace with actuall command from history
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

        // lexical analysis
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

        // execute commands
        INPUTING.store(false, Ordering::Relaxed);
        if commands.len() == 1 {
            // may call built-in functions
            let mut token_iter = commands[0].iter();
            let prog = token_iter.next().cloned().unwrap_or_default();
            match prog.as_str() {
                "" | "history" | "cd" | "export" | "exit" => {
                    let args = token_iter.map(|s| s.to_owned()).collect();
                    do_built_in(&prog, &args, &history);
                    continue;
                }
                _ => (),
            }
        }
        // else build pipes
        let mut subprocess_stdin = Stdio::inherit();
        let mut subprocess_stdout = Stdio::piped();
        let mut command_iter = commands.iter().peekable();
        while let Some(command) = command_iter.next() {
            let last = command_iter.peek().is_none();
            let cur_process_stdout = if last {
                Stdio::inherit()
            } else {
                subprocess_stdout
            };
            match execute_command(command, last, subprocess_stdin, cur_process_stdout) {
                Some((stdin, stdout)) => {
                    (subprocess_stdin, subprocess_stdout) = (stdin, stdout);
                }
                _ => break,
            }
        }

        // wait for all subprocesses
        while match wait() {
            Ok(_) => true,
            _ => false,
        } {}
    }
}

/// execute one command, may be with redirection, like "ls > out"
/// last: if the command is the last one (in pipe)
/// stdin and stdout are suggested by pipe. redirections are prior
fn execute_command(
    command: &[String],
    last: bool,
    mut stdin: Stdio,
    mut stdout: Stdio,
) -> Option<(Stdio, Stdio)> {
    let mut last_command_index = command.len();
    let mut redirect =
        |token: &str, stdio: &mut Stdio, read: bool, write: bool, append: bool| -> () {
            if let Some(index) = command.iter().position(|_token| _token == token) {
                let file_path = command.get(index + 1).expect("error syntax");
                if File::open(file_path).is_err() && (write || append) {
                    File::create(file_path).expect("error create file");
                }
                let file = OpenOptions::new()
                    .read(read)
                    .write(write)
                    .append(append)
                    .open(file_path)
                    .expect("error open file");
                *stdio = Stdio::from(file);
                last_command_index = min(last_command_index, index)
            }
        };
    redirect("<", &mut stdin, true, false, false);
    redirect(">", &mut stdout, false, true, false);
    redirect(">>", &mut stdout, false, true, true);

    let mut token_iter = command[0..last_command_index].iter();
    let prog = token_iter.next().cloned().unwrap_or_default();
    let args: Vec<String> = token_iter.map(|s| s.to_owned()).collect();
    let child = Command::new(&prog)
        .args(&args)
        .stdin(stdin)
        .stdout(stdout)
        .spawn();
    match child {
        Ok(mut child) => {
            if last {
                None
            } else {
                Some((
                    Stdio::from(child.stdout.take().expect("failed to open fd")),
                    Stdio::piped(),
                ))
            }
        }
        _ => {
            println!("failed to start subprocess");
            None
        }
    }
}

/// built-in commands
fn do_built_in(prog: &String, args: &Vec<String>, history: &History) -> () {
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
                    _ => String::new(),
                },
                Some(dir) => dir,
                _ => home.unwrap_or_default(),
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
    let path_err = "Invalid path name";
    let cwd = env::current_dir().expect("Getting current dir failed");
    let home = env::var("HOME");
    let path = match home {
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
    };
    print!("{}{}{}> ", COLOR_GREEN, &path, CLEAR_COLOR);
    io::stdout().flush().expect("error printing prompt");
}
