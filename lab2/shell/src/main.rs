mod subprocess;

use nix::sys::signal::{signal, SigHandler, Signal};
use std::io::{stdin, BufRead, Write};
use std::process::exit;
use std::{env, io};

use crate::subprocess::Subprocess;

extern "C" fn handle_sigint(_: libc::c_int) {
    println!("");
}

fn main() -> ! {
    unsafe { signal(Signal::SIGINT, SigHandler::Handler(handle_sigint)) }
        .expect("Error changing SIGINT handling");

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
                _ => match Subprocess::new(prog, &args.map(|s| s.to_string()).collect()) {
                    Some(subprocess) => {
                        subprocess.wait().expect("Error running subprocess");
                    }
                    _ => println!("Failed to start the program"),
                },
            },
        }
    }
}
