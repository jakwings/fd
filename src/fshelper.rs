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
    // TODO: Provide a flag --real-path for canonicalization of file path?
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

// Path::is_dir() is not guarandteed to be intuitively correct for "." and ".."
// See: https://github.com/rust-lang/rust/issues/45302
pub fn is_dir(path: &Path) -> bool {
    if path.file_name().is_some() {
        path.is_dir()
    } else {
        path.is_dir() && path.canonicalize().is_ok()
    }
}

pub fn is_symlink(path: &Path) -> bool {
    path.symlink_metadata()
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
}

pub fn is_executable(meta: &fs::Metadata) -> bool {
    meta.permissions().mode() & 0o111 != 0
}
