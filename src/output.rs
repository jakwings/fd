use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::Component::{Prefix, RootDir};
use std::path::{self, Path, PathBuf};
use std::process::exit;

use super::nix::sys::signal::Signal::SIGPIPE;

use super::internal::{error, AppOptions};
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
            exit(0x80 + signum);
        } else {
            error(&err.to_string());
        }
    }
}

fn print_entry_colorized(path: &Path, config: &AppOptions, ls_colors: &LsColors) -> io::Result<()> {
    let main_separator = path::MAIN_SEPARATOR.to_string();

    let colorized_separator = ls_colors
        .style_for_indicator(lscolors::Indicator::Directory)
        .map(lscolors::Style::to_ansi_term_style)
        .unwrap_or_default()
        .paint(main_separator.as_bytes());

    // Full path to the current component.
    let mut component_path = PathBuf::new();
    let mut need_separator = false;
    let mut buffer = Vec::new();

    // Traverse the path and colorize each component
    for component in path.components() {
        let compo = component.as_os_str();
        component_path.push(Path::new(compo));

        let style = ls_colors
            .style_for_path(&component_path)
            .map(lscolors::Style::to_ansi_term_style)
            .unwrap_or_default();

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
