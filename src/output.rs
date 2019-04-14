use std::ffi::OsStr;
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::process::exit;

use super::nix::sys::signal::Signal::SIGPIPE;

use super::filter::Action;
use super::internal::{die, warn, AppOptions};
use super::lscolors::{self, LsColors};

pub type Entry = (PathBuf, Vec<Action>);

pub fn print_entry(entry: Entry, config: &AppOptions) {
    let result = if let Some(ref palette) = config.palette {
        print_entry_colorized(entry, palette)
    } else {
        print_entry_uncolorized(entry)
    };

    if let Err(err) = result {
        if err.kind() == io::ErrorKind::BrokenPipe {
            // silently exit
            exit(0x80 + SIGPIPE as i32);
        } else {
            die(&format!("failed to print search result: {}", err));
        }
    }
}

fn print_entry_colorized(entry: Entry, palette: &LsColors) -> io::Result<()> {
    // full path to the last component
    let mut buffer = Vec::new();

    // traverse the path and colorize each component
    for (compo, style) in palette.style_for_path_components(&entry.0) {
        style
            .map(lscolors::Style::to_ansi_term_style)
            .unwrap_or_default()
            .paint(compo.as_os_str().as_bytes())
            .write_to(&mut buffer)?;
    }

    execute_actions(entry, buffer)
}

fn print_entry_uncolorized(entry: Entry) -> io::Result<()> {
    let mut buffer = Vec::new();

    buffer.write(entry.0.as_os_str().as_bytes())?;

    execute_actions(entry, buffer)
}

fn execute_actions(entry: Entry, mut buffer: Vec<u8>) -> io::Result<()> {
    for action in entry.1 {
        match action {
            Action::Print => add_path_terminator(&mut buffer, false),
            Action::Print0 => add_path_terminator(&mut buffer, true),
            _ => continue,
        }
        io::stdout().write_all(buffer.as_slice())?;
        buffer.pop(); // drop terminator
    }

    Ok(())
}

fn add_path_terminator(buffer: &mut Vec<u8>, null_terminated: bool) {
    // TODO: avoid redundant checks
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
        die(&format!(
            "{:?} contains the nul character {:?}",
            OsStr::from_bytes(path),
            OsStr::new("\0")
        ));
    }
    // IDEA: option for turning off warnings?
    // reminder for poor scripts
    if !null_terminated && path.contains(&b'\n') {
        warn(&format!(
            "{:?} contains the line terminator {:?}",
            OsStr::from_bytes(path),
            OsStr::new("\n")
        ));
    }
}
