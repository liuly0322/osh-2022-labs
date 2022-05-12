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
        print_prompt().expect("error print prompt")
    }
}

fn main() -> ! {
    unsafe { signal(Signal::SIGINT, SigHandler::Handler(handle_sigint)) }
        .expect("Error changing SIGINT handler");

    // open or create history file
    let history_file_name =
        env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()) + "/.llysh_history";
    let mut history = History::new(history_file_name).expect("Cannot open history file!");

    loop {
        // prompt message
        INPUTING.store(true, Ordering::Relaxed);
        print_prompt().expect("error print prompt");

        // read line
        let mut command = String::new();
        // EOF handling
        if let Ok(0) = stdin().read_line(&mut command) {
            println!();
            exit(0)
        }

        // if the actuall command is from history
        let command = replace_from_history(&command, &history).unwrap_or(command);
        if command.trim() != history.last().cloned().unwrap_or_default() {
            history.push(&command);
        }

        // seperate commands by pipes
        let tokens: Vec<String> = get_tokens(command);
        let commands: Vec<&[String]> = tokens.split(|token| token == "|").collect();

        // execute commands and concat their stdios with pipes
        INPUTING.store(false, Ordering::Relaxed);
        let mut child_stdin = Stdio::inherit();
        let mut child_stdout = Stdio::piped();
        let mut command_iter = commands.iter().peekable();
        while let Some(command) = command_iter.next() {
            let last = command_iter.peek().is_none();
            let cur_child_stdout = if last { Stdio::inherit() } else { child_stdout };
            match execute_command(command, last, &history, child_stdin, cur_child_stdout) {
                Some((stdin, stdout)) => {
                    (child_stdin, child_stdout) = (stdin, stdout);
                }
                _ => break,
            }
        }

        // wait for all childs
        while matches!(wait(), Ok(_)) {}
    }
}

/// execute one command, may be with redirection, like "ls > out"
/// last: if the command is the last one (in pipe)
/// stdin and stdout are suggested by pipe. redirections are prior
fn execute_command(
    command: &[String],
    last: bool,
    history: &History,
    mut stdin: Stdio,
    mut stdout: Stdio,
) -> Option<(Stdio, Stdio)> {
    let mut last_command_index = command.len();
    let mut redirect =
        |token: &str, stdio: &mut Stdio, read: bool, write: bool, append: bool| -> Option<()> {
            if let Some(index) = command.iter().position(|_token| _token == token) {
                let file_path = command.get(index + 1)?;
                if File::open(file_path).is_err() && (write || append) {
                    File::create(file_path).ok()?;
                }
                let file = OpenOptions::new()
                    .read(read)
                    .write(write)
                    .append(append)
                    .open(file_path)
                    .ok()?;
                *stdio = Stdio::from(file);
                last_command_index = min(last_command_index, index)
            }
            Some(())
        };
    redirect("<", &mut stdin, true, false, false)?;
    redirect(">", &mut stdout, false, true, false)?;
    redirect(">>", &mut stdout, false, true, true)?;

    let mut token_iter = command[0..last_command_index].iter();
    let prog = token_iter.next().cloned().unwrap_or_default();
    let args: Vec<String> = token_iter.map(|s| s.to_owned()).collect();
    if let "" | "history" | "cd" | "export" | "exit" = prog.as_str() {
        if do_built_in(&prog, &args, history).is_none() {
            println!("Error occured in built-in command {}", &prog)
        }
        return None;
    }
    let mut child = Command::new(&prog)
        .args(&args)
        .stdin(stdin)
        .stdout(stdout)
        .spawn()
        .map_err(|_| println!("{}: command not found", &prog))
        .ok()?;
    (!last).then(|| Some((Stdio::from(child.stdout.take()?), Stdio::piped())))?
}

/// built-in commands
fn do_built_in(prog: &str, args: &Vec<String>, history: &History) -> Option<()> {
    match prog {
        "history" => {
            let number = args.get(0)?.parse::<usize>().ok()?;
            let history_size = history.size();
            for i in (0..min(number, history_size)).rev() {
                println!("{:5}  {}", history_size - i, history.rget(i).unwrap())
            }
        }
        "cd" => {
            let home = env::var("HOME").unwrap_or_default();
            let dir = args.get(0).cloned().unwrap_or(home);
            env::set_current_dir(dir).ok()?
        }
        "export" => {
            for arg in args {
                let mut assign = arg.split('=');
                let key = assign.next()?;
                let value = assign.next()?;
                env::set_var(key, value);
            }
        }
        "exit" => {
            exit(0);
        }
        _ => (),
    }
    Some(())
}

/// get tokens for a command
fn get_tokens(command: String) -> Vec<String> {
    command
        .split_whitespace()
        .map(|token| {
            if token.starts_with('$') {
                let key = token.strip_prefix('$').unwrap();
                env::var(key).unwrap_or_default()
            } else if token == "~" || (token.starts_with("~/")) {
                let home = env::var("HOME").unwrap_or_default();
                home + token.strip_prefix('~').unwrap()
            } else {
                token.to_string()
            }
        })
        .collect()
}

/// print prompt message
fn print_prompt() -> Option<()> {
    let cwd = env::current_dir().ok()?;
    let home = env::var("HOME").unwrap_or_default();
    let path = if cwd == Path::new(&home) {
        '~'.to_string()
    } else if !home.is_empty() && cwd.starts_with(&home) {
        "~/".to_string() + cwd.strip_prefix(&home).ok()?.to_str()?
    } else {
        cwd.to_str()?.to_string()
    };
    print!("{}{}{}> ", COLOR_GREEN, &path, CLEAR_COLOR);
    io::stdout().flush().ok()?;
    Some(())
}

/// return the origin command if available
fn replace_from_history(command: &str, history: &History) -> Option<String> {
    let arg = command
        .starts_with('!')
        .then(|| command.strip_prefix('!').unwrap().trim())?;
    let command = if arg.starts_with('!') {
        history.last().cloned()?
    } else {
        let number = arg.parse::<usize>().ok()?;
        history.get(number).cloned()?
    };
    println!("> {}{}{}", COLOR_YELLOW, &command, CLEAR_COLOR);
    Some(command)
}
