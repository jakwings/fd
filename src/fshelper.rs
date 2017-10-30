use std::env::current_dir;
use std::fs;
use std::io;
#[cfg(any(unix, target_os = "redox"))]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub fn to_absolute_path(path: &Path) -> io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        let path = path.strip_prefix(".").unwrap_or(path);
        current_dir().map(|path_buf| path_buf.join(path))
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

#[cfg(any(unix, target_os = "redox"))]
pub fn is_executable(meta: &fs::Metadata) -> bool {
    meta.permissions().mode() & 0o111 != 0
}
