use std::env;
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use super::same_file::is_same_file;

lazy_static! {
    static ref PWD: PathBuf = {
        // NOTE: Unfortunately, current_dir() is always a "real" path on Unix.
        //       Always pass in an absolute path if you don't want that!
        //       Solution: Respect the environment variable "PWD".
        env::current_dir().map(|cwd| {
            env::var_os("PWD").map_or(PathBuf::new(), |path| {
                let pwd = PathBuf::from(path);

                if pwd.is_absolute() && is_same_file(&cwd, &pwd).unwrap_or(false) {
                    pwd
                } else if cwd.is_absolute() {
                    cwd
                } else {
                    PathBuf::new()
                }
            })
        }).unwrap()
    };
    static ref HAS_PWD: bool = !(*PWD).as_os_str().is_empty();
}

pub fn to_absolute_path(path: &Path) -> io::Result<PathBuf> {
    // IDEA: Provide a flag --real-path for canonicalization of file path?
    //       Match real paths and/or output real paths? (affect --include and --exclude?)
    //       Logical: resolve '..' components before symlinks (Windows)
    //       Physical: resolve symlinks as encountered (Unix)
    // NOTE: A path like /root/../compo is considered an absolute path, seriously.
    //       An absolute path is not always a real path (with symlinks fully resolved).
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        let path = path.strip_prefix(".").unwrap_or(path);

        if *HAS_PWD {
            Ok((*PWD).join(path))
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "could not resolve relative path into absolute path",
            ))
        }
    }
}

// Path::exists() and Path::is_dir() do not behave intuitively for "." and ".."
// See: https://github.com/rust-lang/rust/issues/45302
pub fn exists(path: &Path) -> bool {
    path.canonicalize().is_ok()
}

// Only check whether the executable bits are set.
// (is_dir() || is_file() || is_symlink()) may not be true
// (is_block_device() || is_char_device() || is_fifo() || is_socket()) may be true
// It may not be actually executable by execve(2).
pub fn is_executable(meta: &fs::Metadata) -> bool {
    meta.permissions().mode() & 0o111 != 0
}
