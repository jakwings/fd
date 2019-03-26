use std::io;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

use super::counter::Counter;
use super::{error, select_read_to_end, select_write_all, warn, ExecTemplate};

const INTERVAL: u32 = 500 * 1000; // 500 microseconds

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
        if counter.inc() {
            error("scheduler thread aborted");
            return;
        }

        let lock = if let Ok(lock) = receiver.lock() {
            lock
        } else {
            error("scheduler failed to receive data");
            return;
        };

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
                if let Some(ref mut stdin) = child.stdin.take() {
                    let fdin = stdin.as_raw_fd();

                    if select_write_all(&mut counter, fdin, stdin, bytes)?.is_none() {
                        child.kill()?;
                    }
                } else {
                    warn(&format!("{:?}: failed to capture stdin", cmd.prog()));
                }
            }

            let interval = time::Duration::new(0, INTERVAL);
            let result = loop {
                if counter.inc() {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "scheduler thread aborted",
                    ));
                }
                match child.try_wait() {
                    Err(err) => break Err(err),
                    Ok(None) => thread::sleep(interval),
                    Ok(Some(status)) => {
                        if cache_output {
                            let mut buffer = Vec::new();

                            if let Some(ref mut stdout) = child.stdout {
                                let fdout = stdout.as_raw_fd();
                                select_read_to_end(&mut counter, fdout, stdout, &mut buffer)?;
                                let ref mut stdout = &mut io::stdout();
                                let fdout = stdout.as_raw_fd();
                                select_write_all(&mut counter, fdout, stdout, &buffer)?;
                            }

                            buffer.clear();

                            if let Some(ref mut stderr) = child.stderr {
                                let fderr = stderr.as_raw_fd();
                                select_read_to_end(&mut counter, fderr, stderr, &mut buffer)?;
                                let ref mut stderr = &mut io::stderr();
                                let fderr = stderr.as_raw_fd();
                                select_write_all(&mut counter, fderr, stderr, &buffer)?;
                            }
                        }

                        break Ok(status);
                    }
                }
            };

            result
        }) {
            if err.kind() != io::ErrorKind::Other {
                warn(&format!("{:?}: {}", cmd.prog(), err.to_string()));
            } else {
                error(&format!("{:?}: {}", cmd.prog(), err.to_string()));
                return;
            }
        }
    }
}
