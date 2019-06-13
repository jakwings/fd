use std::ffi::OsStr;
use std::fmt::Display;
use std::io::Write;
use std::path::PathBuf;
use std::process;

use super::exec::ExecTemplate;
use super::filter::Chain as FilterChain;
use super::lscolors::LsColors;

#[derive(Debug)]
pub enum Error {
    Message(String),
}

impl std::error::Error for self::Error {}

impl Display for self::Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            self::Error::Message(msg) => write!(f, "{}", msg),
        }
    }
}

impl self::Error {
    pub fn from_str(message: &str) -> self::Error {
        self::Error::Message(message.to_string())
    }
}

pub struct AppOptions {
    // Whether to show warnings about permissions, I/O errors, detected loops, etc.
    pub verbose: bool,

    // Whether the search pattern is Unicode-aware by default.
    pub unicode: bool,

    // Whether to search with a regex pattern.
    pub use_regex: bool,

    // Whether the search is case-sensitive or case-insensitive.
    pub case_insensitive: bool,

    // Whether to match the absolute path or just the base name.
    pub match_full_path: bool,

    // Whether the search results are sorted by pathname.
    pub sort_path: bool,

    // Whether to include dot-files.
    pub dot_files: bool,

    // Whether to respect VCS ignore files (.gitignore, .ignore, etc.).
    pub read_ignore: bool,

    // Whether to multiplex stdin.
    pub multiplex: bool,

    // Whether to follow symbolic links.
    pub follow_symlink: bool,

    // Whether the search is limited for files on the same filesystem.
    pub same_file_system: bool,

    // Whether each search result is terminated with NUL instead of LF when printed.
    pub null_terminator: bool,

    // The maximum search depth for directory traversal.
    pub max_depth: Option<usize>,

    // TODO: min_depth

    // The number of threads to use.
    pub threads: usize,

    // The amount of time for buffering and sorting before streaming the search results.
    pub max_buffer_time: Option<u64>, // milliseconds

    // The starting points for searching.
    pub includes: Vec<PathBuf>,

    // The branches what will be pruned while searching.
    pub excludes: Vec<PathBuf>,

    // The filter for matching file paths.
    pub filter: FilterChain,

    // The command to execute with the search results.
    pub command: Option<ExecTemplate>,

    // The color scheme for output text.
    pub palette: Option<LsColors>,
}

// XXX: https://github.com/rust-lang/rust/issues/41517
//trait Message = Display + ?Sized;

pub fn die(message: &(impl Display + ?Sized)) -> ! {
    error(message);
    process::exit(1)
}

pub fn error(message: &(impl Display + ?Sized)) {
    let stdout = ::std::io::stdout();
    let lock = stdout.lock();

    writeln!(&mut ::std::io::stderr(), "[ff::Error] {}", message).expect("write to stderr");
    drop(lock);
}

pub fn warn(message: &(impl Display + ?Sized)) {
    // XXX: assume both stdout & stderr point to the same file
    let stdout = ::std::io::stdout();
    let lock = stdout.lock();

    writeln!(&mut ::std::io::stderr(), "[ff::Warning] {}", message).expect("write to stderr");
    drop(lock);
}

pub fn int_error(name: &str, num_str: &str, message: &(impl Display + ?Sized)) -> ! {
    die(&format!("{}={:?} {}", name, num_str, message))
}

pub fn int_error_os(name: &str, num_str: &OsStr, message: &(impl Display + ?Sized)) -> ! {
    die(&format!("{}={:?} {}", name, num_str, message))
}
