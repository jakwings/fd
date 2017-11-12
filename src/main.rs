extern crate ansi_term;
extern crate atty;
extern crate clap;
extern crate ctrlc;
extern crate find_mountpoint;
extern crate globset;
extern crate ignore;
extern crate nix;
extern crate num_cpus;
extern crate regex;

mod app;
mod exec;
mod fshelper;
mod glob;
mod internal;
mod lscolors;
mod output;
mod walk;

use std::env;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time;

use atty::Stream;
use regex::bytes::RegexBuilder;

use self::exec::ExecTemplate;
use self::fshelper::{is_dir, to_absolute_path};
use self::glob::GlobBuilder;
use self::internal::{AppOptions, error, int_error, int_error_os};
use self::lscolors::LsColors;
use self::walk::FileType;

fn main() {
    let args = app::build().get_matches();

    let current_dir = PathBuf::from(".");
    if !is_dir(&current_dir) {
        error("cannot get current directory");
    }

    let mut root_dir = match args.value_of_os("DIRECTORY") {
        Some(path_str) => {
            let path = PathBuf::from(path_str);
            if !path_str.is_empty() && path.is_relative()
                && !(path.starts_with(".") || path.starts_with(".."))
            {
                PathBuf::from(".").join(path)
            } else {
                path
            }
        }
        None => current_dir.clone(),
    };
    if !is_dir(&root_dir) {
        error(&format!("{:?} is not a directory", root_dir.as_os_str()));
    }

    let absolute = args.is_present("absolute-path") || root_dir.is_absolute();

    if absolute && root_dir.is_relative() {
        root_dir = to_absolute_path(&root_dir).unwrap();
    }

    let file_type = match args.value_of("file-type") {
        Some("d") | Some("directory") => FileType::Directory,
        Some("f") | Some("file") => FileType::Regular,
        Some("l") | Some("symlink") => FileType::SymLink,
        Some("x") | Some("executable") => FileType::Executable,
        Some(_) | None => if let Some(sym) = args.value_of_os("file-type") {
            error(&format!("unrecognizable file type {:?}", sym))
        } else {
            FileType::Any
        },
    };

    let max_depth = args.value_of("max-depth")
        .map(|num_str| match usize::from_str_radix(num_str, 10) {
            Ok(num) => num,
            Err(err) => int_error("max-depth", num_str, &err.to_string()),
        })
        .or_else(|| {
            args.value_of_os("max-depth").map(|num_str| {
                int_error_os("max-depth", &num_str, "is not an integer");
            })
        });

    let max_buffer_time = args.value_of("max-buffer-time")
        .map(|num_str| match u64::from_str_radix(num_str, 10) {
            Ok(num) => time::Duration::from_millis(num),
            Err(err) => int_error("max-buffer-time", num_str, &err.to_string()),
        })
        .or_else(|| {
            args.value_of_os("max-buffer-time").map(|num_str| {
                int_error_os("max-buffer-time", &num_str, "is not an integer");
            })
        });

    let num_cpu = num_cpus::get();
    let num_thread = args.value_of("threads")
        .map(|num_str| match usize::from_str_radix(num_str, 10) {
            Ok(num) => if num > 0 {
                num
            } else {
                num_cpu
            },
            Err(err) => int_error("threads", num_str, &err.to_string()),
        })
        .or_else(|| {
            args.value_of_os("max-buffer-time").map(|num_str| {
                int_error_os("threads", &num_str, "is not an integer");
            })
        })
        .unwrap_or(num_cpu);

    let colorful = match args.value_of("color") {
        Some("always") => true,
        Some("never") => false,
        _ => atty::is(Stream::Stdout),
    };
    let ls_colors = if colorful {
        // TODO: env::var_os
        Some(
            env::var("LS_COLORS")
                .map(|val| LsColors::from_string(&val))
                .unwrap_or_default(),
        )
    } else {
        None
    };

    let command = args.values_of_os("exec").map(|cmd_args| {
        // `cmd_args` does not contain the terminator ";"
        ExecTemplate::new(&cmd_args.collect())
    });

    let config = AppOptions {
        unicode: args.is_present("unicode"),
        use_regex: args.is_present("use-regex"),
        case_insensitive: args.is_present("ignore-case"),
        match_full_path: args.is_present("full-path"),
        sort_path: args.is_present("sort-path"),
        dot_files: args.is_present("dot-files"),
        read_ignore: !args.is_present("no-ignore"),
        follow_symlink: args.is_present("follow-symlink"),
        same_filesystem: args.is_present("same-filesystem"),
        null_terminator: args.is_present("null_terminator"),
        command: command,
        ls_colors: ls_colors,
        max_buffer_time: max_buffer_time,
        max_depth: max_depth,
        threads: num_thread,
        absolute_path: absolute,
        file_type: file_type,
    };

    let pattern = args.value_of_os("PATTERN");

    let mut builder = if config.use_regex {
        if config.unicode {
            let pattern = OsStr::to_str(pattern.unwrap_or(OsStr::new("^")))
                .unwrap_or_else(|| error("Error: need a UTF-8 encoded pattern"));

            RegexBuilder::new(pattern)
        } else {
            let pattern = escape_pattern(pattern.unwrap_or(OsStr::new("^")))
                .expect("Error: invalid UTF-8 byte sequences found");

            RegexBuilder::new(pattern.as_str())
        }
    } else {
        let pattern = if let Some(p) = pattern {
            // XXX: globset should allow arbitrary bytes?
            p.to_str()
                .unwrap_or_else(|| error("Error: need a UTF-8 encoded pattern"))
        } else {
            if config.match_full_path {
                &"**"
            } else {
                &"*"
            }
        };

        GlobBuilder::new(pattern, config.match_full_path)
    };

    match builder
        .unicode(config.use_regex && config.unicode)
        .case_insensitive(config.case_insensitive)
        .dot_matches_new_line(true)
        .build()
    {
        Ok(re) => walk::scan(&root_dir, Arc::new(re), Arc::new(config)),
        Err(err) => error(&err.to_string()),
    }
}

// XXX: not elegant
fn escape_pattern(pattern: &OsStr) -> Option<String> {
    let mut bytes = Vec::new();

    for c in pattern.as_bytes() {
        let c = *c;

        if c <= 0x1F || c >= 0x7F {
            let mut buff = format!("\\x{:02X}", c);
            bytes.append(unsafe { buff.as_mut_vec() });
        } else {
            bytes.push(c);
        }
    }

    String::from_utf8(bytes).ok()
}
