use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;

use super::ExecTemplate;

/// An event loop that listens for inputs from the `rx` receiver. Each received input will
/// generate a command with the supplied command template. The generated command will then
/// be executed, and this process will continue until the receiver's sender has closed.
pub fn schedule(
    rx: Arc<Mutex<Receiver<PathBuf>>>,
    cmd: Arc<ExecTemplate>,
    out_lock: Arc<Mutex<()>>,
    quitting: Arc<AtomicBool>,
) {
    loop {
        // Create a lock on the shared receiver for this thread.
        let lock = rx.lock().unwrap();

        // Obtain the next path from the receiver, else if the channel
        // has closed, exit from the loop
        let path: PathBuf = match lock.recv() {
            Ok(data) => data,
            Err(_) => break,
        };

        // Drop the lock so that other threads can read from the the receiver.
        drop(lock);

        cmd.generate(&path, Arc::clone(&out_lock), Arc::clone(&quitting))
            .execute();
    }
}
