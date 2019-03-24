use std::fs::FileType as EntryFileType;
use std::io;
use std::option::Option;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::atomic::{self, AtomicBool};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

use super::atty;
use super::ctrlc;
use super::ignore::{self, WalkBuilder, WalkState};
use super::nix::sys::signal::Signal::SIGINT;
use super::regex::bytes::Regex;

use super::counter::Counter;
use super::exec;
use super::fshelper::{is_executable, to_absolute_path};
use super::internal::{error, warn, AppOptions};
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

fn exit_if_sigint(quitting: &Arc<AtomicBool>) {
    if quitting.load(atomic::Ordering::Relaxed) {
        // XXX: https://github.com/Detegr/rust-ctrlc/issues/26
        // XXX: https://github.com/rust-lang/rust/issues/33417
        let signum: i32 = unsafe { ::std::mem::transmute(SIGINT) };

        exit(0x80 + signum);
    }
}

fn spawn_receiver_thread(
    rx: mpsc::Receiver<PathBuf>,
    config: Arc<AppOptions>,
    quitting: Arc<AtomicBool>,
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
                        Err(err) => error(&err.to_string()),
                    };

                drop(lock);

                if aborted {
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
                    exec::schedule(counter, rx, cmd, input, no_stdin, cache_output)
                });

                handles.push(handle);
            }

            // Wait for all threads to exit before exiting the program.
            for h in handles {
                h.join().expect("[Error] unable to process search results");
            }
        } else {
            for value in rx {
                if rx_counter.inc(1) {
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

                        if counter.inc(1) && time::Instant::now() - start > duration {
                            for v in buffer.drain(0..) {
                                tx.send(v).unwrap();
                            }
                            mode = ReceiverMode::Streaming;
                        }
                    }
                    BufferTime::Eternity => {
                        buffer.push(value);
                    }
                },
                ReceiverMode::Streaming => {
                    tx.send(value).unwrap();
                }
            }
        }

        if !buffer.is_empty() {
            // Stable sort is fast enough for nearly sorted items,
            // although it uses 50% more memory than unstable sort.
            // Would parallel sort really help much? Skeptical.
            buffer.sort();

            for value in buffer {
                tx.send(value).unwrap();
            }
        }
    });

    drop(xtx);

    sorter_thread
}

fn spawn_sender_thread(
    tx: mpsc::Sender<PathBuf>,
    root: &Path,
    pattern: Arc<Option<Regex>>,
    config: Arc<AppOptions>,
    quitting: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    // middleware for sorting
    let (xtx, xrx) = mpsc::channel();
    let sorter_thread = spawn_sorter_thread(tx, xrx, Arc::clone(&config));
    let walker = WalkBuilder::new(root)
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
        let pattern = Arc::clone(&pattern);
        let quitting = Arc::clone(&quitting);
        let mut tx_counter = Counter::new(MAX_CNT, Some(quitting));

        Box::new(move |entry_o| {
            if tx_counter.inc(1) {
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
                        // TODO: need to suppress some warnings from deps
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
                            warn(&err.to_string());
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
                        FileType::Executable => {
                            // entry_path.metadata() always follows symlinks
                            if let Ok(meta) = entry_path.metadata() {
                                // only accept likely-execve(2)-able files
                                meta.is_dir()  // this check fails for symlinks
                                    || !(file_type.is_file() || file_type.is_symlink())
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

            if let Some(ref pattern) = *pattern {
                if config.match_full_path {
                    if let Ok(path_buf) = to_absolute_path(&entry_path) {
                        if pattern.is_match(path_buf.as_os_str().as_bytes()) {
                            tx.send(entry_path.to_owned()).unwrap();
                        }
                    } else {
                        error(&format!(
                            "could not get full path of {:?}",
                            entry_path.as_os_str()
                        ));
                    }
                } else {
                    if let Some(os_str) = entry_path.file_name() {
                        if pattern.is_match(os_str.as_bytes()) {
                            tx.send(entry_path.to_owned()).unwrap();
                        }
                    }
                }
            } else {
                tx.send(entry_path.to_owned()).unwrap();
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
pub fn scan(root: &Path, pattern: Arc<Option<Regex>>, config: Arc<AppOptions>) {
    let (tx, rx) = mpsc::channel();

    // A signal to tell the colorizer or the command processor to exit gracefully.
    let quitting = Arc::new(AtomicBool::new(false));
    {
        let atom = Arc::clone(&quitting);
        ctrlc::set_handler(move || {
            atom.store(true, atomic::Ordering::Relaxed);
        })
        .expect("[Error] could not set Ctrl-C handler");
    }

    // Spawn the thread that receives all results through the channel.
    let receiver_thread = spawn_receiver_thread(rx, Arc::clone(&config), Arc::clone(&quitting));

    let sender_thread = spawn_sender_thread(
        tx,
        root,
        pattern,
        Arc::clone(&config),
        Arc::clone(&quitting),
    );

    // Wait for the sender thread to sort & send all results.
    sender_thread
        .join()
        .expect("[Error] unable to produce search results");

    // Wait for the receiver thread to print out all results.
    receiver_thread
        .join()
        .expect("[Error] unable to collect search results");

    exit_if_sigint(&quitting);
}
