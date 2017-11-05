use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{self, AtomicBool};
use std::sync::mpsc::channel;
use std::thread;
use std::time;

use super::ctrlc;
use super::ignore::{self, WalkBuilder};
use super::regex::bytes::Regex;

use super::exec;
use super::fshelper::{is_executable, to_absolute_path};
use super::internal::{AppOptions, error};
use super::output;

/// The receiver thread can either be buffering results or directly streaming to the console.
enum ReceiverMode {
    /// Receiver is still buffering in order to sort the results, if the search finishes fast
    /// enough.
    Buffering,

    /// Receiver is directly printing results to the output.
    Streaming,
}

/// The type of file to search for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FileType {
    Any,
    Directory,
    Regular,
    SymLink,
    Executable,
}

/// Recursively scan the given search path for files/pathnames matching the pattern.
///
/// If the `--exec` argument was supplied, this will create a thread pool for executing
/// jobs in parallel from a given command line and the discovered paths. Otherwise, each
/// path will simply be written to standard output.
pub fn scan(root: &Path, pattern: Arc<Regex>, config: Arc<AppOptions>) {
    let (tx, rx) = channel();
    let threads = config.threads;

    let quitting = Arc::new(AtomicBool::new(false));

    if config.ls_colors.is_some() {
        let atom = quitting.clone();
        ctrlc::set_handler(move || {
            atom.store(true, atomic::Ordering::Relaxed);
        }).expect("Error: cannot set Ctrl-C handler");
    }

    // Spawn the thread that receives all results through the channel.
    let rx_config = Arc::clone(&config);
    let receiver_thread = thread::spawn(move || {
        // This will be set to `Some` if the `--exec` argument was supplied.
        if let Some(ref cmd) = rx_config.command {
            let shared_rx = Arc::new(Mutex::new(rx));

            let cmd = Arc::new(cmd.clone());
            let out_lock = Arc::new(Mutex::new(()));

            // Each spawned job will store it's thread handle in here.
            let mut handles = Vec::with_capacity(threads);
            for _ in 0..threads {
                let rx = Arc::clone(&shared_rx);
                let cmd = Arc::clone(&cmd);
                let out_lock = Arc::clone(&out_lock);
                let quitting = Arc::clone(&quitting);

                // Spawn a job thread that will listen for and execute inputs.
                let handle = thread::spawn(move || exec::schedule(rx, cmd, out_lock, quitting));

                // Push the handle of the spawned thread into the vector for later joining.
                handles.push(handle);
            }

            // Wait for all threads to exit before exiting the program.
            for h in handles {
                h.join().unwrap();
            }
        } else {
            let start = time::Instant::now();

            let mut buffer = vec![];

            // Start in buffering mode
            let mut mode = ReceiverMode::Buffering;

            // Maximum time to wait before we start streaming to the console.
            let max_buffer_time = rx_config
                .max_buffer_time
                .unwrap_or_else(|| time::Duration::from_millis(100));

            for value in rx {
                match mode {
                    ReceiverMode::Buffering => {
                        buffer.push(value);

                        // Have we reached the maximum time?
                        if time::Instant::now() - start > max_buffer_time {
                            // Flush the buffer
                            for v in &buffer {
                                output::print_entry(&v, &rx_config, &quitting);
                            }
                            buffer.clear();

                            // Start streaming
                            mode = ReceiverMode::Streaming;
                        }
                    }
                    ReceiverMode::Streaming => {
                        output::print_entry(&value, &rx_config, &quitting);
                    }
                }
            }

            // If we have finished fast enough (faster than max_buffer_time), we haven't streamed
            // anything to the console, yet. In this case, sort the results and print them:
            if !buffer.is_empty() {
                buffer.sort();
                for value in buffer {
                    output::print_entry(&value, &rx_config, &quitting);
                }
            }
        }
    });

    if !config.sort_path {
        let walker = WalkBuilder::new(root)
            .hidden(!config.dot_files)
            .ignore(config.read_ignore)
            .git_ignore(config.read_ignore)
            .parents(config.read_ignore)
            .git_global(config.read_ignore)
            .git_exclude(config.read_ignore)
            .follow_links(config.follow_symlink)
            .max_depth(config.max_depth)
            .threads(threads)
            .build_parallel();

        // Spawn the sender threads.
        walker.run(|| {
            let config = Arc::clone(&config);
            let pattern = Arc::clone(&pattern);
            let tx = tx.clone();
            let root = root.to_owned();

            Box::new(move |entry_o| {
                let entry = match entry_o {
                    Ok(e) => e,
                    Err(_) => return ignore::WalkState::Continue,
                };
                let entry_path = entry.path();

                if entry_path == root {
                    return ignore::WalkState::Continue;
                }

                if config.file_type != FileType::Any {
                    if let Some(file_type) = entry.file_type() {
                        let to_skip = match config.file_type {
                            FileType::Any => false,
                            FileType::Directory => !file_type.is_dir(),
                            FileType::Regular => !file_type.is_file(),
                            FileType::SymLink => !file_type.is_symlink(),
                            FileType::Executable => {
                                // entry_path.metadata() always follows symlinks
                                if let Ok(meta) = entry_path.metadata() {
                                    meta.is_dir() || !is_executable(&meta)
                                } else if !file_type.is_symlink() {
                                    error(&format!(
                                        "cannot get metadata of {:?}",
                                        entry_path.as_os_str()
                                    ))
                                } else {
                                    false
                                }
                            }
                        };
                        if to_skip {
                            return ignore::WalkState::Continue;
                        }
                    } else {
                        error(&format!(
                            "cannot get file type of {:?}",
                            entry_path.as_os_str()
                        ));
                    }
                }

                if config.match_full_path {
                    if let Ok(path_buf) = to_absolute_path(&entry_path) {
                        if pattern.is_match(path_buf.as_os_str().as_bytes()) {
                            tx.send(entry_path.to_owned())
                                .unwrap_or_else(|err| error(&err.to_string()));
                        }
                    } else {
                        error(&format!(
                            "cannot get full path of {:?}",
                            entry_path.as_os_str()
                        ));
                    }
                } else {
                    if let Some(os_str) = entry_path.file_name() {
                        if pattern.is_match(os_str.as_bytes()) {
                            tx.send(entry_path.to_owned())
                                .unwrap_or_else(|err| error(&err.to_string()));
                        }
                    }
                }

                ignore::WalkState::Continue
            })
        });
    } else {
        let walker = WalkBuilder::new(root)
            .hidden(!config.dot_files)
            .ignore(config.read_ignore)
            .git_ignore(config.read_ignore)
            .parents(config.read_ignore)
            .git_global(config.read_ignore)
            .git_exclude(config.read_ignore)
            .follow_links(config.follow_symlink)
            .max_depth(config.max_depth)
            .sort_by_file_name(OsStr::cmp)
            .threads(1)
            .build();

        // TODO: Dont' Repeat Yourself!
        walker.for_each(|entry_o| {
            let entry = match entry_o {
                Ok(e) => e,
                Err(_) => return,
            };
            let entry_path = entry.path();

            if entry_path == root {
                return;
            }

            if config.file_type != FileType::Any {
                if let Some(file_type) = entry.file_type() {
                    let to_skip = match config.file_type {
                        FileType::Any => false,
                        FileType::Directory => !file_type.is_dir(),
                        FileType::Regular => !file_type.is_file(),
                        FileType::SymLink => !file_type.is_symlink(),
                        FileType::Executable => {
                            // entry_path.metadata() always follows symlinks
                            if let Ok(meta) = entry_path.metadata() {
                                meta.is_dir() || !is_executable(&meta)
                            } else if !file_type.is_symlink() {
                                error(&format!(
                                    "cannot get metadata of {:?}",
                                    entry_path.as_os_str()
                                ))
                            } else {
                                false
                            }
                        }
                    };
                    if to_skip {
                        return;
                    }
                } else {
                    error(&format!(
                        "cannot get file type of {:?}",
                        entry_path.as_os_str()
                    ));
                }
            }

            if config.match_full_path {
                if let Ok(path_buf) = to_absolute_path(&entry_path) {
                    if pattern.is_match(path_buf.as_os_str().as_bytes()) {
                        tx.send(entry_path.to_owned())
                            .unwrap_or_else(|err| error(&err.to_string()));
                    }
                } else {
                    error(&format!(
                        "cannot get full path of {:?}",
                        entry_path.as_os_str()
                    ));
                }
            } else {
                if let Some(os_str) = entry_path.file_name() {
                    if pattern.is_match(os_str.as_bytes()) {
                        tx.send(entry_path.to_owned())
                            .unwrap_or_else(|err| error(&err.to_string()));
                    }
                }
            }
        });
    }

    // Drop the initial sender. If we don't do this, the receiver will block even
    // if all threads have finished, since there is still one sender around.
    drop(tx);

    // Wait for the receiver thread to print out all results.
    receiver_thread.join().unwrap();
}
