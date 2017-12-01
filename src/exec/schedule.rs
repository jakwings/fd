use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Receiver;

use super::ExecTemplate;

// An event loop that listens for inputs from the `receiver`.
// Each received input will generate a command with the supplied command template.
// The generated command will then be executed and process error is irrelevant to ff.
pub fn schedule(receiver: Arc<Mutex<Receiver<PathBuf>>>, template: Arc<ExecTemplate>) {
    loop {
        // Create a lock on the shared receiver for this thread.
        let lock = receiver.lock().unwrap();

        // Obtain the next path from the receiver, else if the channel
        // has closed, exit from the loop
        let path: PathBuf = match lock.recv() {
            Ok(data) => data,
            Err(_) => break,
        };

        // Drop the lock so that other threads can read from the the receiver.
        drop(lock);

        let cmd = template.apply(&path);

        if let Err(err) = cmd.execute() {
            eprintln!("{} {:?}", err.to_string(), cmd.prog());
        }
    }
}
