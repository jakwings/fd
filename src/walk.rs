use std::fs::FileType as EntryFileType;
use std::io;
use std::option::Option;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::atomic::{self, AtomicUsize};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

use super::atty;
use super::ignore::{self, WalkBuilder, WalkState};
use super::signal_hook;

use super::counter::Counter;
use super::exec;
use super::fshelper::{is_executable, to_absolute_path};
use super::internal::{error, fatal, warn, AppOptions};
use super::output;

const MAX_CNT: usize = 500;

#[derive(Clone, Copy, PartialEq)]
enum BufferTime {
    Duration, // End buffering mode after this duration.
    Eternity, // Always buffer the search results.
}

#[derive(Clone, Copy, PartialEq)]
enum ReceiverMode {
    Buffering(BufferTime), // Receiver is still buffering in order to sort the results.
    Streaming,             // Receiver is directly printing search results to the output.
}

#[derive(Clone, Copy, PartialEq)]
pub enum FileType {
    Any,
    Directory,
    Regular,
    SymLink,
    Executable,
}

struct DirEntry<'a> {
    path: &'a Path,
    file_type: Option<EntryFileType>,
}

fn exit_if_sigint(quitting: &Arc<AtomicUsize>) {
    let signum = quitting.load(atomic::Ordering::Relaxed);

    if signum != 0 {
        exit(0x80 + signum as i32);
    }
}

fn spawn_receiver_thread(
    rx: mpsc::Receiver<PathBuf>,
    config: Arc<AppOptions>,
    quitting: Arc<AtomicUsize>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut rx_counter = Counter::new(MAX_CNT, Some(Arc::clone(&quitting)));

        // This will be set to `Some` if the `--exec` argument was supplied.
        if let Some(ref cmd) = config.command {
            // Broadcast the stdin input to all child processes.
            let cached_input = if config.multiplex {
                let stdin = io::stdin();
                let fdin = stdin.as_raw_fd();
                let mut lock = stdin.lock();
                let mut bytes = Vec::new();
                // Do not allow blocking I/O to delay the shutdown of this program.
                // e.g. when waiting for user input.
                let aborted =
                    match exec::select_read_to_end(&mut rx_counter, fdin, &mut lock, &mut bytes) {
                        Ok(None) => true,
                        Ok(Some(_size)) => false,
                        Err(err) => fatal(&err.to_string()),
                    };

                drop(lock);

                if aborted {
                    error("receiver thread failed to read from stdin");
                    return;
                } else {
                    Some(bytes)
                }
            } else {
                None
            };

            let threads = if !config.sort_path { config.threads } else { 1 };
            let cmd = Arc::new(cmd.clone());
            // Enable caching for broadcast, as interactive input may not satisfy all commands.
            let input = Arc::new(cached_input);
            // It is unsafe to interact with mixed output from different commands.
            let no_stdin = threads > 1 && atty::is(atty::Stream::Stdin);
            // Reorder the output only when necessary.
            let cache_output = threads > 1;

            let rx = Arc::new(Mutex::new(rx));
            let mut handles = Vec::with_capacity(threads);

            for _ in 0..threads {
                let rx = Arc::clone(&rx);
                let cmd = Arc::clone(&cmd);
                let input = Arc::clone(&input);
                let quitting = Arc::clone(&quitting);
                let mut counter = Counter::new(MAX_CNT / threads, Some(quitting));
                let handle = thread::spawn(move || {
                    exec::schedule(counter, rx, cmd, input, no_stdin, cache_output);
                });

                handles.push(handle);
            }

            // Wait for all threads to exit before exiting the program.
            for h in handles {
                h.join()
                    .unwrap_or_else(|_| fatal("unable to process search results"));
            }
        } else {
            for value in rx {
                if rx_counter.inc() {
                    error("receiver thread aborted");
                    return;
                }
                output::print_entry(&value, &config);
            }
        }
    })
}

fn spawn_sorter_thread(
    xtx: mpsc::Sender<PathBuf>,
    rx: mpsc::Receiver<PathBuf>,
    config: Arc<AppOptions>,
) -> thread::JoinHandle<()> {
    let tx = xtx.clone();
    let sorter_thread = thread::spawn(move || {
        let max_buffer_time = if atty::is(atty::Stream::Stdout) {
            config.max_buffer_time.unwrap_or(100)
        } else {
            0
        };

        let mut buffer = Vec::new();
        let mut mode = if config.sort_path {
            ReceiverMode::Buffering(BufferTime::Eternity)
        } else if max_buffer_time > 0 && (config.command.is_none() || config.threads == 1) {
            ReceiverMode::Buffering(BufferTime::Duration)
        } else {
            ReceiverMode::Streaming
        };

        let mut counter = Counter::new(MAX_CNT, None);
        let start = time::Instant::now();
        let duration = time::Duration::from_millis(max_buffer_time);

        for value in rx {
            match mode {
                ReceiverMode::Buffering(buf_time) => match buf_time {
                    BufferTime::Duration => {
                        buffer.push(value);

                        if counter.inc() && time::Instant::now() - start > duration {
                            for v in buffer.drain(0..) {
                                if tx.send(v).is_err() {
                                    error("sorter thread failed to send data");
                                    return;
                                }
                            }
                            mode = ReceiverMode::Streaming;
                        }
                    }
                    BufferTime::Eternity => {
                        buffer.push(value);
                    }
                },
                ReceiverMode::Streaming => {
                    if tx.send(value).is_err() {
                        error("sorter thread failed to send data");
                        return;
                    }
                }
            }
        }

        if !buffer.is_empty() {
            // Stable sort is fast enough for nearly sorted items,
            // although it uses 50% more memory than unstable sort.
            // Would parallel sort really help much? Skeptical.
            buffer.sort();

            for value in buffer {
                if tx.send(value).is_err() {
                    error("sorter thread failed to send data");
                    return;
                }
            }
        }
    });

    drop(xtx);

    sorter_thread
}

fn spawn_sender_threads(
    tx: mpsc::Sender<PathBuf>,
    config: Arc<AppOptions>,
    quitting: Arc<AtomicUsize>,
) -> thread::JoinHandle<()> {
    // middleware for sorting
    let (xtx, xrx) = mpsc::channel();
    let sorter_thread = spawn_sorter_thread(tx, xrx, Arc::clone(&config));
    let walker = WalkBuilder::new(&config.root)
        .hidden(!config.dot_files)
        .ignore(config.read_ignore)
        .git_ignore(config.read_ignore)
        .parents(config.read_ignore)
        .git_global(config.read_ignore)
        .git_exclude(config.read_ignore)
        .same_file_system(config.same_file_system)
        .follow_links(config.follow_symlink)
        .max_depth(config.max_depth)
        .threads(config.threads)
        .build_parallel();

    // Spawn the sender threads.
    walker.run(|| {
        let tx = xtx.clone();
        let config = Arc::clone(&config);
        let quitting = Arc::clone(&quitting);
        let mut tx_counter = Counter::new(MAX_CNT, Some(quitting));

        Box::new(move |entry_o| {
            if tx_counter.inc() {
                error("sender thread aborted");
                return WalkState::Quit;
            }

            // https://docs.rs/walkdir/2.2.6/walkdir/struct.DirEntry.html
            let entry = match entry_o {
                Ok(ref entry) => {
                    if entry.depth() != 0 {
                        DirEntry {
                            path: entry.path(),
                            file_type: entry.file_type(),
                        }
                    } else {
                        return WalkState::Continue;
                    }
                }
                Err(ref err) => {
                    let mut broken_symlink = None;

                    // https://docs.rs/walkdir/2.2.6/walkdir/struct.WalkDir.html#method.follow_links
                    // > If a symbolic link is broken or is involved in a loop, an error is yielded.
                    if let ignore::Error::WithPath { path, err: _ } = err {
                        if !err.is_partial() && !path.exists() {
                            let file_type =
                                path.symlink_metadata().map(|meta| meta.file_type()).ok();

                            // Other than symlinks, what may not exist?
                            broken_symlink = Some(DirEntry { path, file_type });
                        }
                    }
                    if broken_symlink.is_some() {
                        broken_symlink.unwrap()
                    } else {
                        if !err.is_partial() || config.verbose {
                            // TODO: need to suppress some warnings from deps
                            //       mkdir -m 000 entrance
                            warn(&err);
                        }
                        return WalkState::Skip;
                    }
                }
            };
            let entry_path = entry.path;

            if config.file_type != FileType::Any {
                if let Some(file_type) = entry.file_type {
                    // only zero or one of is_dir/is_file/is_symlink can be true
                    let to_skip = match config.file_type {
                        FileType::Any => false,
                        FileType::Directory => !file_type.is_dir(),
                        FileType::Regular => !file_type.is_file(),
                        FileType::SymLink => !file_type.is_symlink(),
                        // only accept likely-execve(2)-able files
                        FileType::Executable => {
                            // entry_path.metadata() always follows symlinks
                            if let Ok(meta) = entry_path.metadata() {
                                // also exclude symlinks to directories
                                meta.is_dir()
                                    // exclude character device, block device, sockets, pipes, etc.
                                    || !(file_type.is_file() || file_type.is_symlink())
                                    // with the execute permission file mode bits set
                                    || !is_executable(&meta)
                            } else {
                                if !file_type.is_symlink() {
                                    // permission denied?
                                    warn(&format!(
                                        "could not get metadata of {:?}",
                                        entry_path.as_os_str()
                                    ));
                                } // else: symlinks to non-existent files
                                true
                            }
                        }
                    };
                    if to_skip {
                        return WalkState::Continue;
                    }
                } else {
                    warn(&format!(
                        "could not get file type of {:?}",
                        entry_path.as_os_str()
                    ));
                    return WalkState::Continue;
                }
            }

            if let Some(ref pattern) = config.pattern {
                if config.match_full_path {
                    if let Ok(path_buf) = to_absolute_path(&entry_path) {
                        if pattern.is_match(path_buf.as_os_str().as_bytes()) {
                            if tx.send(entry_path.to_owned()).is_err() {
                                error("sender thread failed to send data");
                                return WalkState::Quit;
                            }
                        }
                    } else {
                        fatal(&format!(
                            "could not get full path of {:?}",
                            entry_path.as_os_str()
                        ));
                    }
                } else {
                    if let Some(os_str) = entry_path.file_name() {
                        if pattern.is_match(os_str.as_bytes()) {
                            if tx.send(entry_path.to_owned()).is_err() {
                                error("sender thread failed to send data");
                                return WalkState::Quit;
                            }
                        }
                    }
                }
            } else {
                if tx.send(entry_path.to_owned()).is_err() {
                    error("sender thread failed to send data");
                    return WalkState::Quit;
                }
            }

            WalkState::Continue
        })
    });

    // Drop the sender. If we don't do this, the receiver will block even
    // if all threads have finished, since there is still one sender around.
    drop(xtx);

    sorter_thread
}

// Recursively scan the given search path for files/pathnames matching the pattern.
//
// If the `--exec` argument was supplied, this will create a thread pool for executing
// jobs in parallel from a given command line and the discovered paths. Otherwise, each
// path will simply be written to standard output.
pub fn scan(config: Arc<AppOptions>) {
    let (tx, rx) = mpsc::channel();

    // A signal to tell the colorizer or the command processor to exit gracefully.
    let quitting = Arc::new(AtomicUsize::new(0));

    signal_hook::flag::register_usize(
        signal_hook::SIGINT,
        Arc::clone(&quitting),
        signal_hook::SIGINT as usize,
    )
    .unwrap_or_else(|_| fatal("could not set SIGINT handler"));

    signal_hook::flag::register_usize(
        signal_hook::SIGTERM,
        Arc::clone(&quitting),
        signal_hook::SIGTERM as usize,
    )
    .unwrap_or_else(|_| fatal("could not set SIGTERM handler"));

    // Spawn the threads that receive or send results through the channel.
    let recv_handle = spawn_receiver_thread(rx, Arc::clone(&config), Arc::clone(&quitting));
    let send_handle = spawn_sender_threads(tx, Arc::clone(&config), Arc::clone(&quitting));

    // Wait for the sender thread to sort & send all results.
    send_handle
        .join()
        .unwrap_or_else(|_| fatal("unable to produce search results"));

    // Wait for the receiver thread to print out all results.
    recv_handle
        .join()
        .unwrap_or_else(|_| fatal("unable to collect search results"));

    exit_if_sigint(&quitting);
}
