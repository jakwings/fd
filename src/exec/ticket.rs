use std::io;
use std::process::{Command, exit};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{self, AtomicBool};

use super::ExecCommand;
use super::SIGINT;

/// A state that offers access to executing a generated command.
pub struct ExecTicket {
    command: ExecCommand,
    out_lock: Arc<Mutex<()>>,
    quitting: Arc<AtomicBool>,
}

impl ExecTicket {
    pub fn new(
        command: ExecCommand,
        out_lock: Arc<Mutex<()>>,
        quitting: Arc<AtomicBool>,
    ) -> ExecTicket {
        ExecTicket {
            command,
            out_lock,
            quitting,
        }
    }

    /// Executes the command stored within the ticket,
    /// and clearing the command's buffer when finished.
    #[cfg(target_os = "redox")]
    pub fn execute(&self) {
        use std::io::Write;
        use std::process::Stdio;

        self.exit_if_sigint();

        // Spawn a shell with the supplied command.
        let cmd = Command::new(self.command.prog())
            .args(self.command.args())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        // Then wait for the command to exit, if it was spawned.
        match cmd {
            Ok(output) => {
                // While this lock is active, this thread will be the only thread allowed
                // to write it's outputs.
                let _lock = self.out_lock.lock().unwrap();

                let stdout = io::stdout();
                let stderr = io::stderr();

                let _ = stdout.lock().write_all(&output.stdout);
                let _ = stderr.lock().write_all(&output.stderr);
            }
            Err(err) => eprintln!("{} {:?}", err.to_string(), self.command.prog()),
        }
    }

    #[cfg(all(unix, not(target_os = "redox")))]
    pub fn execute(&self) {
        use super::libc::{close, dup2, pipe, STDERR_FILENO, STDOUT_FILENO};
        use std::fs::File;
        use std::os::unix::io::FromRawFd;
        use std::os::unix::process::CommandExt;

        self.exit_if_sigint();

        // Initial a pair of pipes that will be used to
        // pipe the std{out,err} of the spawned process.
        let mut stdout_fds = [0; 2];
        let mut stderr_fds = [0; 2];

        unsafe {
            pipe(stdout_fds.as_mut_ptr());
            pipe(stderr_fds.as_mut_ptr());
        }

        // Spawn a shell with the supplied command.
        let cmd = Command::new(self.command.prog())
            .args(self.command.args())
            // Configure the pipes accordingly in the child.
            .before_exec(move || unsafe {
                // Redirect the child's std{out,err} to the write ends of our pipe.
                dup2(stdout_fds[1], STDOUT_FILENO);
                dup2(stderr_fds[1], STDERR_FILENO);

                // Close all the fds we created here, so EOF will be sent when the program exits.
                close(stdout_fds[0]);
                close(stdout_fds[1]);
                close(stderr_fds[0]);
                close(stderr_fds[1]);
                Ok(())
            })
            .spawn();

        // Open the read end of the pipes as `File`s.
        let (mut pout, mut perr) = unsafe {
            // Close the write ends of the pipes in the parent
            close(stdout_fds[1]);
            close(stderr_fds[1]);
            (
                // But create files from the read ends.
                File::from_raw_fd(stdout_fds[0]),
                File::from_raw_fd(stderr_fds[0]),
            )
        };

        match cmd {
            Ok(mut child) => {
                let _ = child.wait();

                // Create a lock to ensure that this thread has exclusive access to writing.
                let _lock = self.out_lock.lock().unwrap();

                // And then write the outputs of the process until EOF is sent to each file.
                let stdout = io::stdout();
                let stderr = io::stderr();
                let _ = io::copy(&mut pout, &mut stdout.lock());
                let _ = io::copy(&mut perr, &mut stderr.lock());
            }
            Err(err) => eprintln!("{} {:?}", err.to_string(), self.command.prog()),
        }
    }

    fn exit_if_sigint(&self) {
        if self.quitting.load(atomic::Ordering::Relaxed) {
            let signum: i32 = unsafe { ::std::mem::transmute(SIGINT) };
            exit(0x80 + signum);
        }
    }
}
