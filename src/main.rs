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
mod filter;
mod foss;
mod fshelper;
mod glob;
mod internal;
mod lscolors;
mod output;
mod pattern;
mod walk;

use std::path::PathBuf;
use std::sync::Arc;

use self::exec::ExecTemplate;
use self::filter::{Chain as FilterChain, FileType, Filter};
use self::fshelper::{is_dir, to_absolute_path};
use self::internal::{die, int_error, int_error_os, AppOptions};
use self::lscolors::LsColors;
use self::pattern::PatternBuilder;

fn main() {
    let args = app::build().get_matches();

    let current_dir = PathBuf::from(".");
    if !is_dir(&current_dir) {
        die("could not get current directory");
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
        die(&format!("{:?} is not a directory", root_dir.as_os_str()));
    }

    let absolute = args.is_present("absolute-path") || root_dir.is_absolute();

    if absolute && root_dir.is_relative() {
        root_dir = to_absolute_path(&root_dir).unwrap();
    }

    let file_type = args
        .value_of_os("file-type")
        .map(|symbol| FileType::from_str(&symbol).unwrap_or_else(|err| die(&err)));

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
    let palette = if colorful {
        Some(LsColors::from_env().unwrap_or_default())
    } else {
        None
    };

    let command = args.values_of_os("exec").map(|cmd_args| {
        if args.occurrences_of("PATTERN") > 1 {
            die("forbidden to use filter chain and --exec at the same time");
        }
        // `cmd_args` does not contain the terminator ";"
        ExecTemplate::new(&cmd_args.collect())
    });

    let mut config = AppOptions {
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
        filter: FilterChain::new(Filter::Anything, false),
        command: command,
        palette: palette,
        max_buffer_time: max_buffer_time,
        max_depth: max_depth,
        threads: num_thread,
        absolute_path: absolute,
    };

    let pattern = args.values_of_os("PATTERN").as_mut().map(|values| {
        if args.occurrences_of("PATTERN") == 1 {
            let source = values.next().unwrap();

            PatternBuilder::new(source)
                .use_regex(config.use_regex)
                .unicode(config.unicode)
                .case_insensitive(config.case_insensitive)
                .match_full_path(config.match_full_path)
                .build()
                .map(|pattern| {
                    if config.match_full_path {
                        FilterChain::new(Filter::Path(pattern), false)
                    } else {
                        FilterChain::new(Filter::Name(pattern), false)
                    }
                })
                .unwrap_or_else(|err| {
                    die(&format!(
                        "failed to build search pattern {:?}:\n{}",
                        source, err
                    ))
                })
        } else {
            FilterChain::from_args(values, &config)
                .unwrap_or_else(|err| die(&format!("failed to build filter chain:\n{}", err)))
        }
    });

    if let Some(file_type) = file_type {
        config.filter = config.filter.and(Filter::Type(file_type), false);
    }
    if let Some(pattern) = pattern {
        config.filter = config.filter.and(Filter::Chain(pattern), false);
    }
    config.filter = FilterChain::reduce(config.filter);

    walk::scan(Arc::new(config));
}
