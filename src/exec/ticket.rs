use std::process::{Command, exit};
use std::sync::Arc;
use std::sync::atomic::{self, AtomicBool};

use super::ExecCommand;
use super::SIGINT;

pub struct ExecTicket {
    command: ExecCommand,
    quitting: Arc<AtomicBool>,
}

impl ExecTicket {
    pub fn new(command: ExecCommand, quitting: Arc<AtomicBool>) -> ExecTicket {
        ExecTicket { command, quitting }
    }

    pub fn execute(&self) {
        self.exit_if_sigint();

        let cmd = Command::new(self.command.prog())
            .args(self.command.args())
            .spawn();

        if let Err(err) = cmd.and_then(|mut child| child.wait()) {
            eprintln!("{} {:?}", err.to_string(), self.command.prog());
        }
    }

    fn exit_if_sigint(&self) {
        if self.quitting.load(atomic::Ordering::Relaxed) {
            let signum: i32 = unsafe { ::std::mem::transmute(SIGINT) };
            exit(0x80 + signum);
        }
    }
}
