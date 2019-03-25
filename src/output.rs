use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::process::exit;

use super::nix::sys::signal::Signal::SIGPIPE;

use super::internal::{fatal, AppOptions};
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

    if config.null_terminator {
        buffer.push(b'\0');
    } else {
        buffer.push(b'\n');
    };

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
