use std::ffi::OsStr;
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::process::exit;

use super::nix::sys::signal::Signal::SIGPIPE;

use super::internal::{fatal, warn, AppOptions};
use super::lscolors::{self, LsColors};

pub fn print_entry(entry: &Path, config: &AppOptions) {
    let result = if let Some(ref ls_colors) = config.ls_colors {
        print_entry_colorized(entry, config, ls_colors)
    } else {
        print_entry_uncolorized(entry, config)
    };

    if let Err(err) = result {
        if err.kind() == io::ErrorKind::BrokenPipe {
            let signum: i32 = unsafe { ::std::mem::transmute(SIGPIPE) };
            // XXX: should not be silent for SIGTERM
            exit(0x80 + signum);
        } else {
            fatal(&err);
        }
    }
}

fn print_entry_colorized(path: &Path, config: &AppOptions, ls_colors: &LsColors) -> io::Result<()> {
    // full path to the last component
    let mut buffer = Vec::new();

    // traverse the path and colorize each component
    for (compo, style) in ls_colors.style_for_path_components(path) {
        style
            .map(lscolors::Style::to_ansi_term_style)
            .unwrap_or_default()
            .paint(compo.as_os_str().as_bytes())
            .write_to(&mut buffer)?;
    }
    add_path_terminator(&mut buffer, config.null_terminator);

    io::stdout().write_all(&buffer.into_boxed_slice())
}

fn print_entry_uncolorized(path: &Path, config: &AppOptions) -> io::Result<()> {
    let mut buffer = Vec::new();

    buffer.write(&path.as_os_str().as_bytes())?;
    add_path_terminator(&mut buffer, config.null_terminator);

    io::stdout().write_all(&buffer.into_boxed_slice())
}

fn add_path_terminator(buffer: &mut Vec<u8>, null_terminated: bool) {
    check_path(buffer, null_terminated);

    if null_terminated {
        buffer.push(b'\0');
    } else {
        buffer.push(b'\n');
    }
}

fn check_path(path: &Vec<u8>, null_terminated: bool) {
    // int execve(const char *path, char *const argv[], char *const envp[]);
    if path.contains(&b'\0') {
        fatal(&format!(
            "{:?} contains the nul character {:?}",
            OsStr::from_bytes(path),
            OsStr::new("\0")
        ));
    }
    // TODO: option for turning off warnings?
    // reminder for poor scripts
    if !null_terminated && path.contains(&b'\n') {
        warn(&format!(
            "{:?} contains the line terminator {:?}",
            OsStr::from_bytes(path),
            OsStr::new("\n")
        ));
    }
}
