extern crate atty;
extern crate clap;
extern crate globset;
extern crate ignore;
#[macro_use]
extern crate lazy_static;
extern crate nix;
extern crate num_cpus;
extern crate regex;
extern crate same_file;
extern crate signal_hook;

mod app;
mod counter;
mod exec;
mod foss;
mod fshelper;
mod glob;
mod internal;
mod lscolors;
mod output;
mod walk;

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::sync::Arc;

use regex::bytes::RegexBuilder;

use self::exec::ExecTemplate;
use self::fshelper::{is_dir, to_absolute_path};
use self::glob::GlobBuilder;
use self::internal::{error, fatal, int_error, int_error_os, warn, AppOptions};
use self::lscolors::LsColors;
use self::walk::FileType;

fn main() {
    let args = app::build().get_matches();

    let current_dir = PathBuf::from(".");
    if !is_dir(&current_dir) {
        fatal("could not get current directory");
    }

    let mut root_dir = match args.value_of_os("DIRECTORY") {
        Some(path_str) => {
            let path = PathBuf::from(path_str);
            if !path_str.is_empty()
                && path.is_relative()
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
        fatal(&format!("{:?} is not a directory", root_dir.as_os_str()));
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
        Some(_) | None => {
            if let Some(sym) = args.value_of_os("file-type") {
                fatal(&format!("unrecognizable file type {:?}", sym))
            } else {
                FileType::Any
            }
        }
    };

    let max_depth = args
        .value_of("max-depth")
        .map(|num_str| match usize::from_str_radix(num_str, 10) {
            Ok(num) => num,
            Err(err) => int_error("max-depth", num_str, &err),
        })
        .or_else(|| {
            args.value_of_os("max-depth").map(|num_str| {
                int_error_os("max-depth", &num_str, "is not an integer");
            })
        });

    let max_buffer_time = args
        .value_of("max-buffer-time")
        .map(|num_str| match u64::from_str_radix(num_str, 10) {
            Ok(num) => num,
            Err(err) => int_error("max-buffer-time", num_str, &err),
        })
        .or_else(|| {
            args.value_of_os("max-buffer-time").map(|num_str| {
                int_error_os("max-buffer-time", &num_str, "is not a non-negative integer");
            })
        });

    let num_cpu = num_cpus::get();
    let num_thread = args
        .value_of("threads")
        .map(|num_str| match usize::from_str_radix(num_str, 10) {
            Ok(num) => {
                if num > 0 {
                    num
                } else {
                    num_cpu
                }
            }
            Err(err) => int_error("threads", num_str, &err),
        })
        .or_else(|| {
            args.value_of_os("threads").map(|num_str| {
                int_error_os("threads", &num_str, "is not an integer");
            })
        })
        .unwrap_or(num_cpu)
        .max(1);

    let colorful = match args.value_of("color") {
        Some("always") => true,
        Some("never") => false,
        _ => atty::is(atty::Stream::Stdout),
    };
    let ls_colors = if colorful {
        Some(LsColors::from_env().unwrap_or_default())
    } else {
        None
    };

    let command = args.values_of_os("exec").map(|cmd_args| {
        // `cmd_args` does not contain the terminator ";"
        ExecTemplate::new(&cmd_args.collect())
    });

    let use_regex = args.is_present("use-regex");
    let unicode = args.is_present("unicode");
    let match_full_path = args.is_present("full-path");
    let case_insensitive = args.is_present("ignore-case");
    let pattern = args.value_of_os("PATTERN").map(|pattern| {
        let mut builder = if use_regex {
            let pattern = if unicode {
                OsStr::to_os_string(pattern)
                    .into_string()
                    .unwrap_or_else(|_| fatal("need a UTF-8 encoded pattern"))
            } else {
                escape_pattern(pattern)
                    .unwrap_or_else(|| fatal("invalid UTF-8 byte sequences found"))
            };

            // XXX: strange conformance to UTF-8
            //      (?u)π or (?u:π) doesn't match π without --unicode?
            //      (?-u:π) is not allowed with --unicode?
            RegexBuilder::new(&pattern)
        } else {
            let pattern =
                OsStr::to_str(pattern).unwrap_or_else(|| fatal("need a UTF-8 encoded pattern"));

            // XXX: strange conformance to UTF-8
            GlobBuilder::new(pattern, unicode, match_full_path)
        };

        builder
            .unicode(unicode)
            .case_insensitive(case_insensitive)
            .dot_matches_new_line(true)
            .build()
            .unwrap_or_else(|err| fatal(&err))
    });

    let config = AppOptions {
        verbose: args.is_present("verbose"),
        unicode: args.is_present("unicode"),
        use_regex: args.is_present("use-regex"),
        case_insensitive: args.is_present("ignore-case"),
        match_full_path: args.is_present("full-path"),
        sort_path: args.is_present("sort-path"),
        dot_files: args.is_present("dot-files"),
        read_ignore: !args.is_present("no-ignore"),
        multiplex: args.is_present("multiplex"),
        follow_symlink: args.is_present("follow-symlink"),
        same_file_system: args.is_present("same-file-system"),
        null_terminator: args.is_present("null-terminator"),
        root: root_dir,
        pattern: pattern,
        command: command,
        ls_colors: ls_colors,
        max_buffer_time: max_buffer_time,
        max_depth: max_depth,
        threads: num_thread,
        absolute_path: absolute,
        file_type: file_type,
    };

    walk::scan(Arc::new(config));
}

// XXX: not elegant
// The regex crate can't help much: https://github.com/rust-lang/regex/issues/426
// The man asked my use case again and again, but I found this guy case-insensitive.
fn escape_pattern(pattern: &OsStr) -> Option<String> {
    let mut bytes = Vec::new();

    for c in pattern.as_bytes() {
        let c = *c;

        if c <= 0x1F || c >= 0x7F {
            let buff = format!("\\x{:02X}", c);

            bytes.append(&mut buff.into_bytes());
        } else {
            bytes.push(c);
        }
    }

    String::from_utf8(bytes).ok()
}
