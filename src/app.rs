use std::collections::HashMap;

use clap::{App, AppSettings, Arg};

struct Help {
    short: &'static str,
    long: &'static str,
}

macro_rules! doc {
    ($map:expr, $name:expr, $short:expr) => {
        doc!($map, $name, $short, $short)
    };
    ($map:expr, $name:expr, $short:expr, $long:expr) => {
        $map.insert(
            $name,
            Help {
                short: $short,
                // use dirty hack to separate lines by an empty line
                long: concat!($long, "\n "),
            },
        );
    };
}

pub fn build() -> App<'static, 'static> {
    let help = get_help();
    let arg = |name| {
        Arg::with_name(name)
            .help(help[name].short)
            .long_help(help[name].long)
            .hide_default_value(true)
            .empty_values(true)  // may imply .takes_value(true)
            .takes_value(false)
    };

    App::new("ff")
        .global_settings(&[
            AppSettings::AllowInvalidUtf8,
            AppSettings::ArgsNegateSubcommands,
            AppSettings::ColoredHelp,
            AppSettings::DeriveDisplayOrder,
            AppSettings::DontCollapseArgsInUsage,
            AppSettings::HidePossibleValuesInHelp,
            AppSettings::UnifiedHelpMessage,
            AppSettings::VersionlessSubcommands,
        ])
        .unset_settings(&[AppSettings::StrictUtf8])
        .max_term_width(80)
        .version(env!("CARGO_PKG_VERSION"))
        .usage("ff [OPTIONS] [<DIRECTORY> [PATTERN]]")
        .about("A simple and fast utility for file search on Unix commandline.")
        .help_message(
            "Prints help information. \
             Use --help to show details and full list of options.",
        )
        .after_help(
            "NOTE: If the value of environment variable PWD \
             is the path of a symlink pointing to the current working directory, \
             it will be used for resolving the absolute path of a relative path.",
        )
        .arg(
            arg("use-glob")
                .long("glob")
                .short("g")
                .overrides_with("use-regex")
                .hidden_short_help(true),
        )
        .arg(
            arg("use-regex")
                .long("regex")
                .short("r")
                .overrides_with("use-glob"),
        )
        .arg(arg("unicode").long("unicode").short("u"))
        .arg(
            arg("ignore-case")
                .long("ignore-case")
                .short("i")
                .overrides_with("case-sensitive"),
        )
        .arg(
            arg("case-sensitive")
                .long("case-sensitive")
                .short("s")
                .overrides_with("ignore-case")
                .hidden_short_help(true),
        )
        .arg(
            arg("full-path")
                .long("full-path")
                .short("p")
                .hidden_short_help(true),
        )
        .arg(
            arg("follow-symlink")
                .long("follow")
                .short("L")
                .hidden_short_help(true),
        )
        .arg(
            arg("same-file-system")
                .long("mount")
                .short("M")
                .hidden_short_help(true),
        )
        .arg(
            arg("null-terminator")
                .long("print0")
                .short("0")
                .hidden_short_help(true),
        )
        .arg(
            arg("absolute-path")
                .long("absolute-path")
                .short("A")
                .hidden_short_help(true),
        )
        .arg(
            arg("sort-path")
                .long("sort-path")
                .short("S")
                .hidden_short_help(true),
        )
        .arg(arg("dot-files").long("all").short("a"))
        .arg(arg("no-ignore").long("no-ignore").short("I"))
        .arg(
            arg("multiplex")
                .long("multiplex")
                .short("m")
                .hidden_short_help(true),
        )
        .arg(
            arg("file-type")
                .long("type")
                .short("t")
                .takes_value(true)
                .value_name("filetype")
                .hidden_short_help(true),
        )
        .arg(
            arg("max-depth")
                .long("max-depth")
                .short("d")
                .takes_value(true)
                .value_name("number")
                .hidden_short_help(true),
        )
        .arg(
            arg("color")
                .long("color")
                .short("c")
                .takes_value(true)
                .value_name("when")
                .possible_values(&["auto", "never", "always"])
                .hidden_short_help(true),
        )
        .arg(
            arg("threads")
                .long("threads")
                .short("j")
                .takes_value(true)
                .value_name("number")
                .hidden_short_help(true),
        )
        .arg(
            arg("max-buffer-time")
                .long("max-buffer-time")
                .takes_value(true)
                .value_name("milliseconds")
                .hidden_short_help(true),
        )
        .arg(
            arg("exec")
                .long("exec")
                .short("x")
                .allow_hyphen_values(true)
                .value_name("program [argument]... [;]")
                .value_terminator(";")
                .min_values(1)
                .hidden_short_help(true),
        )
        .arg(arg("verbose").long("verbose").short("v"))
        .arg(arg("DIRECTORY").default_value(".").empty_values(false))
        .arg(arg("PATTERN"))
}

// TODO upstream: Remove trailing spaces in --help message.
//                https://github.com/kbknapp/clap-rs/issues/1094
fn get_help() -> HashMap<&'static str, Help> {
    let mut help = HashMap::new();

    doc!(
        help,
        "unicode",
        "Match UTF-8 scalar values [default: match bytes]",
        "Turn on Unicode support for search patterns. Character classes are not limited to ASCII. \
         Only valid UTF-8 byte sequences can be matched by the search pattern."
    );

    doc!(
        help,
        "use-glob",
        "Search with a glob pattern. [default]",
        "Match the whole file path with a glob pattern. This is the default behavior."
    );

    doc!(
        help,
        "use-regex",
        "Search with a regex pattern. [default: glob]",
        "Match the whole file path with a regex pattern. [default: glob]"
    );

    doc!(
        help,
        "ignore-case",
        "Case-insensitive search. [default: case-sensitive]",
        "Perform a case-insensitive search. This overrides --case-sensitive."
    );

    doc!(
        help,
        "case-sensitive",
        "Case-sensitive search. [default]",
        "Perform a case-sensitive search. This is the default behavior."
    );

    doc!(
        help,
        "full-path",
        "Match full paths. [default: match filename]",
        "Match the absolute path instead of the filename or directory name."
    );

    doc!(
        help,
        "follow-symlink",
        "Follow symbolic links.",
        "Follow symlinks and traverse the symlinked directories."
    );

    doc!(
        help,
        "same-file-system",
        "Do not descend into directories on other file systems.",
        "Do not descend into directories on other file systems, \
         as a symlink or normal directory may lead to a file on another file system."
    );

    doc!(
        help,
        "null-terminator",
        "Terminate each search result with NUL.",
        "Each search result is terminated with NUL instead of LF when printed."
    );

    doc!(
        help,
        "absolute-path",
        "Output absolute paths instead of relative paths.",
        "Relative paths for output are transformed into absolute paths."
    );

    doc!(
        help,
        "sort-path",
        "Sort the results by pathname.",
        "The search results will be sorted by pathname before output. \
         This option will also force --exec to use a single thread for processing. \
         Sort by lexicographically comparing the byte strings of path components \
         (not comparing the whole pathnames directly)."
    );

    doc!(
        help,
        "dot-files",
        "Include dot-files in the search.",
        "All files and directories are searched. By default, files and directories \
         of which the names start with a dot \".\" are ignored in the search. \
         Files ignored by patterns in .(git)ignore files are still excluded."
    );

    doc!(
        help,
        "no-ignore",
        "Do not respect .(git)ignore files.",
        "Show search results from files and directories that would otherwise be ignored by \
         .(git)ignore files."
    );

    doc!(
        help,
        "multiplex",
        "All executed commands receive the same input.",
        "Multiplex stdin of this program so that every executed command shares the same input. \
         Interactive input is disabled by caching, even if the commands run sequentially."
    );

    doc!(
        help,
        "file-type",
        "Filter by type: d,directory, f,file, l,symlink, x,executable",
        concat!(
            "Filter the search by type: [default: no filter]\n",
            "    directory or d: directories\n",
            "         file or f: regular files\n",
            "      symlink or l: symbolic links\n",
            "   executable or x: executable files"
        )
    );

    doc!(
        help,
        "max-depth",
        "Set maximum search depth. [default: none]",
        "Limit the directory traversal to a given depth."
    );

    doc!(
        help,
        "color",
        "When to use colors: auto, never, always [default: auto]",
        concat!(
            "Declare when to use color for the pattern match output:\n",
            "      auto: use colors for interactive console [default]\n",
            "     never: do not use colorized output\n",
            "    always: always use colorized output"
        )
    );

    doc!(
        help,
        "threads",
        "Set number of threads for searching and command execution.",
        "The number of threads to use for searching and command execution.\n\
         0 means [default: number of available CPU cores]"
    );

    doc!(
        help,
        "max-buffer-time",
        "Set time (in milliseconds) for buffering and sorting.",
        "The amount of time (in milliseconds) for the search results to be buffered and sorted \
         before streaming."
    );

    doc!(
        help,
        "exec",
        "Execute the given command for each search result.",
        "Run the given command for each search result, which can be represented by a pair of \
         braces {} in the command. If the command does not contain any {}, then a {} will be \
         appended as an argument to the program. A single semicolon ; will terminate the \
         argument list.\n\
         With --threads=1 commands will run sequentially. When multi-threading is enabled and \
         multiplexing is not enabled, commands will not receive input from the terminal. \
         If not running with a single thread, each output of the command will be buffered, \
         reordered (printed to stdout before stderr) and synchronized to avoid overlap."
    );

    doc!(
        help,
        "verbose",
        "Warn about I/O errors, file permissions, symlink loops, etc.",
        "Show warnings about file permissions, loops caused by symlinks, I/O errors, \
         invalid file content, etc."
    );

    doc!(
        help,
        "DIRECTORY",
        "The root directory for the search. [optional]",
        "The directory where the search is rooted. \
         If omitted, search the current working directory."
    );

    doc!(
        help,
        "PATTERN",
        "The search pattern, a regex or glob pattern. [optional]\n\
         The default values for regex and glob are ^ and * respectively."
    );

    help
}
