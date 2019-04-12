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

use std::path::{Path, PathBuf};
use std::sync::Arc;

use self::exec::ExecTemplate;
use self::filter::{Chain as FilterChain, FileType, Filter};
use self::fshelper::{exists, to_absolute_path};
use self::internal::{die, int_error, int_error_os, AppOptions};
use self::lscolors::LsColors;
use self::pattern::PatternBuilder;

fn normalize(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    let os_str = path.as_os_str();

    if os_str.is_empty() {
        die(&format!("{:?} is not a file or directory", os_str));
    } else if path.is_relative() && !(path.starts_with(".") || path.starts_with("..")) {
        PathBuf::from(".").join(path)
    } else {
        path.to_path_buf()
    }
}

fn main() {
    let args = app::build().get_matches();

    let absolute = args.is_present("absolute-path");

    let current_dir = PathBuf::from(".");
    let mut root_dirs = Vec::with_capacity(1);

    match args.value_of_os("DIRECTORY") {
        Some(os_str) => root_dirs.push(normalize(os_str)),
        None => root_dirs.push(current_dir.clone()),
    }

    args.values_of_os("include").map(|values| {
        root_dirs.append(&mut values.map(normalize).collect());
    });

    root_dirs.iter_mut().for_each(|dir| {
        if !exists(dir) {
            if *dir == current_dir {
                die("could not get current directory")
            } else {
                die(&format!("{:?} is not a file or directory", dir.as_os_str()));
            }
        } else if absolute {
            *dir = to_absolute_path(dir).unwrap_or_else(|err| die(&err));
        }
    });

    let mut pruned_dirs = Vec::new();

    args.values_of_os("exclude").map(|values| {
        pruned_dirs.append(&mut values.map(normalize).collect());
    });
    pruned_dirs.sort_unstable();
    pruned_dirs.dedup();

    if absolute {
        for dir in pruned_dirs.iter_mut() {
            *dir = to_absolute_path(dir).unwrap_or_else(|err| die(&err));
        }
    }

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
        includes: root_dirs,
        excludes: pruned_dirs,
        filter: FilterChain::default(),
        command: command,
        palette: palette,
        max_buffer_time: max_buffer_time,
        max_depth: max_depth,
        threads: num_thread,
    };

    let file_type = args.values_of_os("file-type").map(|values| {
        values.fold(FilterChain::default().not(), |chain, value| {
            let ftype = FileType::from_str(value).unwrap_or_else(|err| die(&err));

            chain.and(Filter::Type(ftype), true)
        })
    });

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

    if let Some(chain) = file_type {
        config.filter = config.filter.and(Filter::Chain(chain), false);
    }
    if let Some(chain) = pattern {
        config.filter = config.filter.and(Filter::Chain(chain), false);
    }
    config.filter = FilterChain::reduce(config.filter);

    walk::scan(Arc::new(config));
}
