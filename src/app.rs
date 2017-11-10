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
        $map.insert($name, Help {
            short: $short,
            // use dirty hack to separate lines by an empty line
            long: concat!($long, "\n ")
        });
    };
}

// TODO upstream:
//     Only show advanced options with the *long* flag --help.
//     https://github.com/kbknapp/clap-rs/issues/1064
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
            AppSettings::NextLineHelp,
            AppSettings::UnifiedHelpMessage,
            AppSettings::VersionlessSubcommands,
        ])
        .unset_settings(&[AppSettings::StrictUtf8])
        .max_term_width(80)
        .version(env!("CARGO_PKG_VERSION"))
        .usage("ff [OPTIONS] [<DIRECTORY> [PATTERN]]")
        .help_message("Prints help information. Use --help for more details.")
        .arg(
            arg("use-glob")
                .long("glob")
                .short("g")
                .overrides_with("use-regex"),
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
                .overrides_with("ignore-case"),
        )
        .arg(arg("full-path").long("full-path").short("p"))
        .arg(arg("follow-symlink").long("follow").short("L"))
        .arg(arg("same-filesystem").long("mount").short("M"))
        .arg(arg("null_terminator").long("print0").short("0"))
        .arg(arg("absolute-path").long("absolute-path").short("A"))
        .arg(arg("sort-path").long("sort-path").short("S"))
        .arg(arg("dot-files").long("all").short("a"))
        .arg(arg("no-ignore").long("no-ignore").short("I"))
        .arg(
            arg("file-type")
                .long("type")
                .short("t")
                .takes_value(true)
                .value_name("filetype"),
        )
        .arg(
            arg("max-depth")
                .long("max-depth")
                .short("d")
                .takes_value(true)
                .value_name("number"),
        )
        .arg(
            arg("color")
                .long("color")
                .short("c")
                .takes_value(true)
                .value_name("when")
                .possible_values(&["auto", "never", "always"]),
        )
        .arg(
            arg("threads")
                .long("threads")
                .short("j")
                .takes_value(true)
                .value_name("number"),
        )
        .arg(
            arg("max-buffer-time")
                .long("max-buffer-time")
                .takes_value(true)
                .value_name("milliseconds"),
        )
        .arg(
            arg("exec")
                .long("exec")
                .short("x")
                .allow_hyphen_values(true)
                .value_name("program [argument]... [;]")
                .value_terminator(";")
                .min_values(1),
        )
        .arg(arg("DIRECTORY").default_value(".").empty_values(false))
        .arg(arg("PATTERN").default_value(""))
}

// TODO upstream:
//     Remove trailing spaces in --help message.
//     https://github.com/kbknapp/clap-rs/issues/1094
fn get_help() -> HashMap<&'static str, Help> {
    let mut help = HashMap::new();

    doc!(
        help,
        "unicode",
        "Match UTF-8 scalar values [default: match bytes]",
        "Turn on Unicode support for regex patterns. Character classes are not limited to ASCII. \
         Only valid UTF-8 byte sequences can be matched by the search pattern."
    );

    doc!(
        help,
        "use-glob",
        "Search with a glob pattern. [default: regex]",
        "Match the whole file path with a glob pattern. [default: regex]"
    );

    doc!(
        help,
        "use-regex",
        "Search with a regex pattern. [default]",
        "Match the whole file path with a regex pattern. This is the default behavior."
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
        "same-filesystem",
        "Do not descend into directories on other filesystems.",
        "Do not descend into directories on other filesystems, \
         as a symlink may point to a directory on another filesystem."
    );

    doc!(
        help,
        "null_terminator",
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
        concat!(
            "The number of threads to use for searching and command execution.\n",
            "0 means [default: number of available CPU cores]"
        )
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
         argument list."
    );

    doc!(
        help,
        "DIRECTORY",
        "The root directory for the filesystem search. [optional]",
        "The directory where the filesystem search is rooted. \
         If omitted, search the current working directory."
    );

    doc!(
        help,
        "PATTERN",
        concat!(
            "The search pattern, a regex or glob pattern. [optional]\n",
            "The default values for regex and glob are ^ and * respectively."
        )
    );

    help
}
