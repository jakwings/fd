use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::{self, Path, PathBuf};
use std::path::Component::{Prefix, RootDir};
use std::process::exit;
use std::sync::Arc;
use std::sync::atomic::{self, AtomicBool};

use super::ansi_term::Style;
use super::nix::sys::signal::Signal::{SIGINT, SIGPIPE};

use super::fshelper::{is_executable, is_symlink};
use super::internal::{AppOptions, error};
use super::lscolors::LsColors;

pub fn print_entry(entry: &Path, config: &AppOptions, quitting: &Arc<AtomicBool>) {
    let result = if let Some(ref ls_colors) = config.ls_colors {
        print_entry_colorized(entry, config, ls_colors, quitting)
    } else {
        print_entry_uncolorized(entry, config)
    };

    if let Err(err) = result {
        if err.kind() == io::ErrorKind::BrokenPipe {
            let signum: i32 = unsafe { ::std::mem::transmute(SIGPIPE) };
            exit(0x80 + signum);
        } else {
            error(&err.to_string());
        }
    }
}

fn print_entry_colorized(
    path: &Path,
    config: &AppOptions,
    ls_colors: &LsColors,
    quitting: &Arc<AtomicBool>,
) -> io::Result<()> {
    let main_separator = path::MAIN_SEPARATOR.to_string();

    let default_style = Style::default();
    let colorized_separator = ls_colors.directory.paint(main_separator.as_bytes());

    // Full path to the current component.
    let mut component_path = PathBuf::new();
    let mut need_separator = false;
    let mut buffer = Vec::new();

    // Traverse the path and colorize each component
    for component in path.components() {
        let compo = component.as_os_str();
        component_path.push(Path::new(compo));

        let style = get_path_style(&component_path, &ls_colors).unwrap_or(&default_style);

        if need_separator {
            colorized_separator.write_to(&mut buffer)?;
        }
        style.paint(compo.as_bytes()).write_to(&mut buffer)?;

        // assigns later because RootDir (MAIN_SEPARATOR) could have been printed before
        need_separator = match component {
            Prefix(_) | RootDir => false,
            _ => true,
        };
    }

    if config.null_terminator {
        buffer.push(b'\0');
    } else {
        buffer.push(b'\n');
    };

    // SIGINT: Exit before or after colorized output is completely written out.
    if quitting.load(atomic::Ordering::Relaxed) {
        // XXX: https://github.com/Detegr/rust-ctrlc/issues/26
        // XXX: https://github.com/rust-lang/rust/issues/33417
        let signum: i32 = unsafe { ::std::mem::transmute(SIGINT) };
        exit(0x80 + signum);
    }

    io::stdout().write_all(&buffer.into_boxed_slice())
}

fn print_entry_uncolorized(path: &Path, config: &AppOptions) -> io::Result<()> {
    let mut buffer = Vec::new();

    buffer.write(&path.as_os_str().as_bytes())?;

    if config.null_terminator {
        buffer.push(b'\0');
    } else {
        buffer.push(b'\n');
    }

    io::stdout().write_all(&buffer.into_boxed_slice())
}

fn get_path_style<'a>(path: &Path, ls_colors: &'a LsColors) -> Option<&'a Style> {
    if is_symlink(path) {
        return if path.exists() {
            Some(&ls_colors.symlink)
        } else {
            Some(&ls_colors.inexistent)
        };
    }

    if path.is_dir() {
        return Some(&ls_colors.directory);
    }

    let metadata = path.metadata();
    if metadata.map(|meta| is_executable(&meta)).unwrap_or(false) {
        return Some(&ls_colors.executable);
    }

    let filename_style = path.file_name()
        .and_then(|name| ls_colors.filenames.get(name));
    if filename_style.is_some() {
        return filename_style;
    }

    let extension_style = path.extension()
        .and_then(|ext| ls_colors.extensions.get(ext));
    if extension_style.is_some() {
        return extension_style;
    }

    None
}
