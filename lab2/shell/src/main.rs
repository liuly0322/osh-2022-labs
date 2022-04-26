use nix::errno::Errno;
use nix::sys::signal::{signal, SigHandler, Signal};
use nix::sys::wait::wait;
use std::io::{stdin, BufRead, Write};
use std::process::{exit, Child, Command};
use std::{env, io};

extern "C" fn handle_sigint(_: libc::c_int) {
    print!("\n");
    match wait() {
        Err(Errno::ECHILD) => print!("% "),
        _ => (),
    }
    io::stdout().flush().expect("error printing prompt");
}

fn main() -> ! {
    unsafe { signal(Signal::SIGINT, SigHandler::Handler(handle_sigint)) }
        .expect("Error changing SIGINT handler");

    loop {
        // prompt message
        print!("% ");
        io::stdout().flush().expect("error printing prompt");

        let mut cmd = String::new();
        match stdin().lock().read_line(&mut cmd) {
            Ok(0) => exit(0),
            _ => (),
        }
        let mut args = cmd.split_whitespace();
        let prog = args.next();

        match prog {
            None => (),
            Some(prog) => match prog {
                "cd" => {
                    let dir = args.next().expect("No enough args to set current dir");
                    env::set_current_dir(dir).expect("Changing current dir failed");
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
                _ => match subprocess(prog, &args.map(|s| s.to_string()).collect()) {
                    Some(_) => loop {
                        match wait() {
                            Err(Errno::ECHILD) => break,
                            _ => (),
                        }
                    },
                    _ => println!("Failed to start the program"),
                },
            },
        }
    }
}

// returns Some(Child) if successful
fn subprocess(target: &str, args: &Vec<String>) -> Option<Child> {
    let mut command = Command::new(target);
    let command = command.args(args);
    let command = command.spawn().ok()?;
    Some(command)
}
