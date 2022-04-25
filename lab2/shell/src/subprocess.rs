use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::Pid;
use std::process::Child;
use std::process::Command;

pub struct Subprocess {
    child: Child,
}

impl Subprocess {
    // returns Some(Subprocess) if successful
    pub fn new(target: &str, args: &Vec<String>) -> Option<Subprocess> {
        let mut command = Command::new(target);
        let command = command.args(args);
        let command = command.spawn().ok()?;
        Some(Subprocess { child: command })
    }

    pub fn pid(&self) -> Pid {
        nix::unistd::Pid::from_raw(self.child.id() as i32)
    }

    // call waitpid on the subprocess
    pub fn wait(&self) -> Result<WaitStatus, nix::Error> {
        Ok(waitpid(self.pid(), None)?)
    }
}
