use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Receiver;

use super::{ExecTemplate, warn};

// Each received input will generate a command with the supplied command template.
// Then execute the generated command and wait for the child process.
// Resource would get exhausted if we keep spawning new processes without waiting for the old ones.
pub fn schedule(receiver: Arc<Mutex<Receiver<PathBuf>>>, template: Arc<ExecTemplate>) {
    loop {
        let lock = receiver.lock().expect("Error: failed to acquire lock");
        let path: PathBuf = match lock.recv() {
            Ok(data) => data,
            Err(_) => break,
        };

        drop(lock);

        let cmd = template.apply(&path);

        if let Err(err) = cmd.execute().and_then(|mut child| child.wait()) {
            warn(&format!("{:?}: {}", cmd.prog(), err.to_string()));
        }
    }
}
