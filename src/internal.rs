use std::ffi::OsStr;
use std::io::Write;
use std::process;
use std::time;

use super::exec::ExecTemplate;
use super::lscolors::LsColors;
use super::walk::FileType;

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

    // Whether the search results are absolute paths.
    pub absolute_path: bool,

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
    pub same_filesystem: bool,

    // Whether each search result is terminated with NUL instead of LF when printed.
    pub null_terminator: bool,

    // The type of files to search for.
    pub file_type: FileType,

    // The maximum search depth for directory traversal.
    pub max_depth: Option<usize>,

    // The number of threads to use.
    pub threads: usize,

    // The amount of time for buffering and sorting before streaming the search results.
    pub max_buffer_time: Option<time::Duration>,

    // The command to execute with the search results.
    pub command: Option<ExecTemplate>,

    // The color scheme for output text.
    pub ls_colors: Option<LsColors>,
}

pub fn error(message: &str) -> ! {
    writeln!(&mut ::std::io::stderr(), "[Error] {}", message).expect("write to stderr");
    process::exit(1)
}

pub fn warn(message: &str) {
    writeln!(&mut ::std::io::stderr(), "[Warning] {}", message).expect("write to stderr");
}

pub fn int_error(name: &str, num_str: &str, message: &str) -> ! {
    error(&format!("{}={:?} {}", name, num_str, message))
}

pub fn int_error_os(name: &str, num_str: &OsStr, message: &str) -> ! {
    error(&format!("{}={:?} {}", name, num_str, message))
}
