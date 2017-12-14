use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{self, AtomicBool};
use std::sync::mpsc::Receiver;

use super::{ExecTemplate, warn, set_nonblocking, try_write_all};

// Each received input will generate a command with the supplied command template.
// Then execute the generated command and wait for the child process.
// Resource would get exhausted if we keep spawning new processes without waiting for the old ones.
pub fn schedule(
    quitting: Arc<AtomicBool>,
    receiver: Arc<Mutex<Receiver<PathBuf>>>,
    template: Arc<ExecTemplate>,
    cached_input: Arc<Option<Vec<u8>>>,
    no_tty: bool,
) {
    loop {
        if quitting.load(atomic::Ordering::Relaxed) {
            return;
        }

        let lock = receiver.lock().expect("[Error] failed to acquire lock");

        let path: PathBuf = match lock.recv() {
            Ok(data) => data,
            Err(_) => break,
        };

        drop(lock);

        let cmd = template.apply(&path);
        let stdin = if cached_input.is_some() {
            Stdio::piped()
        } else if no_tty {
            Stdio::null()
        } else {
            Stdio::inherit()
        };

        if let Err(err) = cmd.execute(stdin).and_then(|mut child| {
            if let Some(ref bytes) = *cached_input {
                if let Some(mut stdin) = child.stdin.take() {
                    // Not necessary, but unblocking I/O helps to exit earlier.
                    let is_nonblocking = if let Err(msg) = set_nonblocking(&stdin) {
                        warn(msg);
                        false
                    } else {
                        true
                    };

                    if is_nonblocking {
                        if try_write_all(&quitting, &mut stdin, bytes)?.is_none() {
                            child.kill()?;
                        }
                    } else {
                        stdin.write_all(bytes)?;
                    }
                } else {
                    warn(&format!("{:?}: failed to capture stdin", cmd.prog()));
                }
            }

            child.wait()
        }) {
            warn(&format!("{:?}: {}", cmd.prog(), err.to_string()));
        }
    }
}
