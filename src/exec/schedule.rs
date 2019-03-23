use std::io::{self, Write};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

use super::super::counter::Counter;
use super::{select_write_all, warn, ExecTemplate};

// Each received input will generate a command with the supplied command template.
// Then execute the generated command and wait for the child process.
// Resource would get exhausted if we keep spawning new processes without waiting for the old ones.
pub fn schedule(
    mut counter: Counter,
    receiver: Arc<Mutex<Receiver<PathBuf>>>,
    template: Arc<ExecTemplate>,
    cached_input: Arc<Option<Vec<u8>>>,
    no_stdin: bool,
    cache_output: bool,
) {
    loop {
        if counter.inc(1) {
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
        } else if no_stdin {
            Stdio::null()
        } else {
            Stdio::inherit()
        };

        let (stdout, stderr) = if cache_output {
            (Stdio::piped(), Stdio::piped())
        } else {
            (Stdio::inherit(), Stdio::inherit())
        };

        if let Err(err) = cmd.execute(stdin, stdout, stderr).and_then(|mut child| {
            if let Some(ref bytes) = *cached_input {
                if let Some(mut stdin) = child.stdin.take() {
                    let fdin = stdin.as_raw_fd();

                    if select_write_all(&mut counter, fdin, &mut stdin, bytes)?.is_none() {
                        child.kill()?;
                    }
                } else {
                    warn(&format!("{:?}: failed to capture stdin", cmd.prog()));
                }
            }

            if cache_output {
                child.wait_with_output().and_then(|output| {
                    // Even select() cannot help to avoid reordering.
                    io::stdout().write_all(&output.stdout)?;
                    io::stderr().write_all(&output.stderr)?;
                    Ok(output.status)
                })
            } else {
                child.wait()
            }
        }) {
            warn(&format!("{:?}: {}", cmd.prog(), err.to_string()));
        }
    }
}
