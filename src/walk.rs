use std::io;
use std::option::Option;
use std::os::unix::io::AsRawFd;
use std::path::Path;
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
use super::internal::{die, error, warn, AppOptions};
use super::output;

const MAX_CNT: usize = 500;

#[derive(Clone, Copy)]
enum BufferTime {
    Duration, // End buffering mode after this duration.
    Eternity, // Always buffer the search results.
}

#[derive(Clone, Copy)]
enum ReceiverMode {
    Buffering(BufferTime), // Receiver is still buffering in order to sort the results.
    Streaming,             // Receiver is directly printing search results to the output.
}

pub struct DirEntry<'a> {
    pub path: &'a Path,
    pub file_type: Option<std::fs::FileType>,
}

fn exit_if_sigint(quitting: &Arc<AtomicUsize>) {
    let signum = quitting.load(atomic::Ordering::Relaxed);

    if signum != 0 {
        exit(0x80 + signum as i32);
    }
}

fn calc_send_threads(threads: usize, sort_path: bool) -> usize {
    if sort_path {
        (threads - 1).max(1) // minus receiver thread
    } else if threads > 1 {
        (threads / 2).max(2)
    } else {
        threads.max(1)
    }
}

fn calc_recv_threads(threads: usize, sort_path: bool) -> usize {
    if sort_path {
        1
    } else if threads > 1 {
        (threads / 2).max(2)
    } else {
        threads.max(1)
    }
}

fn spawn_receiver_threads(
    rx: mpsc::Receiver<output::Entry>,
    config: Arc<AppOptions>,
    quitting: Arc<AtomicUsize>,
) -> Vec<thread::JoinHandle<()>> {
    // This will be set to `Some` if the `--exec` argument was supplied.
    if let Some(ref cmd) = config.command {
        // Broadcast the stdin input to all child processes.
        let cached_input = if config.multiplex {
            let mut rx_counter = Counter::new(MAX_CNT, Some(Arc::clone(&quitting)));
            let stdin = io::stdin();
            let fdin = stdin.as_raw_fd();
            let mut lock = stdin.lock();
            let mut bytes = Vec::new();
            // Do not allow blocking I/O to delay the shutdown of this program.
            // e.g. when waiting for user input.
            match exec::select_read_to_end(&mut rx_counter, fdin, &mut lock, &mut bytes) {
                Ok(Some(_size)) => (),
                Ok(None) => {
                    error("receiver thread aborted");
                    return Vec::new();
                }
                Err(err) => die(&format!(
                    "receiver thread failed to read from stdin: {}",
                    err
                )),
            }
            drop(lock);

            Some(bytes)
        } else {
            None
        };

        let threads = calc_recv_threads(config.threads, config.sort_path);

        let cmd = Arc::new(cmd.clone());
        // Enable caching for broadcast, as interactive input may not satisfy all commands.
        let input = Arc::new(cached_input);
        // It is unsafe to interact with mixed output from different commands.
        let no_stdin = threads > 1 && atty::is(atty::Stream::Stdin);
        // Reorder the output only when necessary.
        let cache_output = threads > 1;

        let mut handles = Vec::with_capacity(threads);
        let rx = Arc::new(Mutex::new(rx));

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

        return handles;
    }

    Vec::new()
}

fn spawn_sender_threads(
    tx: mpsc::Sender<output::Entry>,
    config: Arc<AppOptions>,
    quitting: Arc<AtomicUsize>,
) {
    let threads = calc_send_threads(config.threads, config.sort_path);
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
        .threads(threads)
        // the non-parallel version can output first few sorted results earlier
        // and make less buffering but the total time used is 4 times longer
        .build_parallel();

    // Spawn the sender threads.
    walker.run(|| {
        let tx = tx.clone();
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

            let actions = config.filter.apply(&entry, &config);

            if !actions.is_empty() {
                if tx.send((entry.path.to_owned(), actions)).is_err() {
                    error("sender thread failed to send data");
                    return WalkState::Quit;
                }
            }

            WalkState::Continue
        })
    });

    // Drop the sender. If we don't do this, the receiver will block even
    // if all threads have finished, since there is still one sender around.
    drop(tx);
}

#[inline(always)]
fn print_or_pipe(
    print_mode: bool,
    value: output::Entry,
    tx: &mpsc::Sender<output::Entry>,
    config: &Arc<AppOptions>,
) -> bool {
    if print_mode {
        // NOTE: impossible to gracefully exit for `ff --color=always | break_pipe`
        output::print_entry(value, config);
    } else {
        if tx.send(value).is_err() {
            error("sorter thread failed to send data");
            return false;
        }
    }
    return true;
}

fn spawn_sorter_thread(
    handles: Vec<thread::JoinHandle<()>>,
    tx: mpsc::Sender<output::Entry>,
    rx: mpsc::Receiver<output::Entry>,
    config: Arc<AppOptions>,
    quitting: Arc<AtomicUsize>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let print_mode = handles.is_empty();

        let max_buffer_time = if atty::is(atty::Stream::Stdout) {
            config.max_buffer_time.unwrap_or(100)
        } else {
            0
        };

        let threads = calc_recv_threads(config.threads, config.sort_path);
        let mut buffer = Vec::new();
        let mut mode = if config.sort_path {
            ReceiverMode::Buffering(BufferTime::Eternity)
        } else if max_buffer_time > 0 && (config.command.is_none() || threads == 1) {
            ReceiverMode::Buffering(BufferTime::Duration)
        } else {
            ReceiverMode::Streaming
        };

        let mut rx_counter = Counter::new(MAX_CNT, Some(Arc::clone(&quitting)));
        let mut counter = Counter::new(MAX_CNT, None);
        let start = time::Instant::now();
        let duration = time::Duration::from_millis(max_buffer_time);

        for value in rx {
            if rx_counter.inc() {
                error("sorter thread aborted");
                return;
            }
            match mode {
                ReceiverMode::Buffering(buf_time) => match buf_time {
                    BufferTime::Duration => {
                        buffer.push(value);

                        if counter.inc() && time::Instant::now() - start > duration {
                            for value in buffer.drain(0..) {
                                if !print_or_pipe(print_mode, value, &tx, &config) {
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
                    if !print_or_pipe(print_mode, value, &tx, &config) {
                        return;
                    }
                }
            }
        }

        if !buffer.is_empty() {
            // Stable sort is fast enough for nearly sorted items,
            // although it uses 50% more memory than unstable sort.
            // Stable sort is also needed for ordered actions.
            // Would parallel sort really help much? Skeptical.
            buffer.sort_by_key(|value| value.0.to_owned());

            for value in buffer {
                if rx_counter.inc() {
                    error("sorter thread aborted");
                    return;
                }
                if !print_or_pipe(print_mode, value, &tx, &config) {
                    return;
                }
            }
        }

        drop(tx);

        // Wait for the --exec threads to print out all results.
        for handle in handles {
            if handle.join().is_err() {
                die("failed to process search results");
            }
        }
    })
}

// Recursively scan the given search path for files/pathnames matching the pattern.
//
// If the `--exec` argument was supplied, this will create a thread pool for executing
// jobs in parallel from a given command line and the discovered paths. Otherwise, each
// path will simply be written to standard output.
pub fn scan(config: Arc<AppOptions>) {
    // A signal to tell the colorizer or the command processor to exit gracefully.
    let quitting = Arc::new(AtomicUsize::new(0));

    signal_hook::flag::register_usize(
        signal_hook::SIGINT,
        Arc::clone(&quitting),
        signal_hook::SIGINT as usize,
    )
    .unwrap_or_else(|_| die("could not set SIGINT handler"));

    signal_hook::flag::register_usize(
        signal_hook::SIGTERM,
        Arc::clone(&quitting),
        signal_hook::SIGTERM as usize,
    )
    .unwrap_or_else(|_| die("could not set SIGTERM handler"));

    let (tx, rx) = mpsc::channel();
    // middleware for sorting
    let (xtx, xrx) = mpsc::channel();

    let handles = spawn_receiver_threads(xrx, Arc::clone(&config), Arc::clone(&quitting));

    let handle = spawn_sorter_thread(handles, xtx, rx, Arc::clone(&config), Arc::clone(&quitting));

    // blocking current thread because of WalkParallel::run()
    spawn_sender_threads(tx, Arc::clone(&config), Arc::clone(&quitting));

    if handle.join().is_err() {
        die("failed to process search results");
    }

    exit_if_sigint(&quitting);
}
