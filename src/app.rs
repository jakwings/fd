use std::collections::HashMap;

use super::clap::{App, AppSettings, Arg};

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
        .version_message("Print version information.")
        .usage("ff [OPTIONS] [<DIRECTORY> [PATTERN | FILTER CHAIN]]")
        .about("A simple and fast utility for file search on Unix commandline.")
        .help_message(
            "Print help information.\n\
             Use --help to show details and full list of options.",
        )
        .after_help(
            "NOTE: If the value of environment variable PWD \
             is the path of a symlink pointing to the current working directory, \
             it is used for resolving the absolute path of a relative path.",
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
                .alias("case-insensitive")
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
        .arg(
            arg("DIRECTORY")
                .default_value(".")
                .empty_values(false)
                .next_line_help(true),
        )
        .arg(
            arg("PATTERN")
                .value_name("PATTERN | FILTER CHAIN")
                .multiple(true)
                .next_line_help(true),
        )
}

// TODO upstream: Remove trailing spaces in --help message.
//                https://github.com/kbknapp/clap-rs/issues/1094
fn get_help() -> HashMap<&'static str, Help> {
    let mut help = HashMap::new();

    doc!(
        help,
        "unicode",
        "Match UTF-8 scalar values",
        "Turn on Unicode support for search patterns.\n\
         \n\
         Character classes are not limited to ASCII. \
         Only valid UTF-8 byte sequences can be matched by the search pattern."
    );

    doc!(
        help,
        "use-glob",
        "Search with a glob pattern. [default]",
        "Match file paths with a glob pattern.\n\
         This is the default behavior."
    );

    doc!(
        help,
        "use-regex",
        "Search with a regex pattern.",
        "Match file paths with a regex pattern."
    );

    doc!(
        help,
        "ignore-case",
        "Case-insensitive search.",
        "Perform a case-insensitive search."
    );

    doc!(
        help,
        "case-sensitive",
        "Case-sensitive search. [default]",
        "Perform a case-sensitive search.\n\
         This is the default behavior."
    );

    doc!(
        help,
        "full-path",
        "Match the full path of a file.",
        "Match the absolute path instead of only the file name or directory name."
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
        "Do not descend into directories on another file system.",
        "Do not descend into directories on another disk or partition, \
         as a symlink or normal directory may lead to a file on another file system."
    );

    doc!(
        help,
        "null-terminator",
        "Terminate each search result with a NUL character.",
        "Each search result is terminated with a NUL character instead of a newline (LF) \
         when printed.\n\
         \n\
         This option does not affect --exec."
    );

    doc!(
        help,
        "absolute-path",
        "Output absolute paths instead of relative paths.",
        "Relative paths for output are transformed into absolute paths.\n\
         \n\
         An absolute path may not be the real path due to symlinks."
    );

    doc!(
        help,
        "sort-path",
        "Sort the results by pathname.",
        "The search results are sorted by pathname before output.\n\
         \n\
         Sort by lexicographically comparing the byte strings of path components. \
         The search depth is also taken into comsideration.\n\
         \n\
         This option also forces --exec to use a single thread for processing."
    );

    doc!(
        help,
        "dot-files",
        "Include dot-files in the search.",
        "All files and directories are searched.\n\
         \n\
         By default, files and directories \
         of which the names start with a dot \".\" are ignored in the search.\n\
         \n\
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
        "Multiplex stdin of this program so that every executed command shares the same input.\n\
         \n\
         Interactive input is disabled by caching, even if the commands run sequentially."
    );

    doc!(
        help,
        "file-type",
        "Filter by type: d,directory, f,file, l,symlink, x,executable",
        concat!(
            "Filter the search by type (case-insensitive): [default: any]\n",
            "\n",
            "    directory or d: directories\n",
            "         file or f: regular files\n",
            "      symlink or l: symbolic links\n",
            "   executable or x: executable files\n",
            "\n",
            "Executable files are regular files with execute permission bits set \
             or are symlinks pointing to the former, which means they are likely \
             programs that can be loaded and run on the operating system."
        )
    );

    doc!(
        help,
        "max-depth",
        "Set maximum search depth. [default: unlimited]",
        "Limit the directory traversal to a given depth."
    );

    doc!(
        help,
        "color",
        "When to use colors: auto, never, always [default: auto]",
        concat!(
            "Declare when to use color for the pattern match output:\n",
            "\n",
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
         \n\
         0 means [default: number of available CPU cores]"
    );

    doc!(
        help,
        "max-buffer-time",
        "Set time (in milliseconds) for buffering and sorting.",
        "The amount of time (in milliseconds) \
         for the search results to be buffered and sorted before streaming.\n\
         \n\
         This option is mostly a stub for testing purpose."
    );

    doc!(
        help,
        "exec",
        "Execute the given command for each search result.",
        "Run the given command for each search result.\n\
         \n\
         The search result can be represented by a pair of braces {} in the command. \
         If the command does not contain any {}, \
         then a {} is appended as an argument to the program. \
         A single semicolon ; terminates the argument list.\n\
         \n\
         With --threads=1 commands are run sequentially. \
         If multi-threading is enabled and multiplexing is not enabled, \
         commands do not receive input from an interactive console.\n\
         \n\
         If not running with a single thread, each output of the command is buffered, \
         reordered (printed to stdout before stderr) and synchronized to avoid overlap."
    );

    doc!(
        help,
        "verbose",
        "Warn about I/O errors, permission, symlink loops, etc.",
        "Show warnings about file permissions, loops caused by symlinks, I/O errors, \
         invalid file content, etc."
    );

    doc!(
        help,
        "DIRECTORY",
        "The root directory for the search. [optional]\n\
         If omitted, search the current working directory."
    );

    doc!(
        help,
        "PATTERN",
        "A regex or glob pattern for matching files. [optional]\n\
         The default patterns for regex and glob are ^ and * respectively.\n\
         It can also be a chain of filters. Use --help for details.",
        concat!(
            "A regex or glob pattern for matching files. [optional]\n",
            "The default patterns for regex and glob are ^ and * respectively.\n",
            "\n",
            "The expression can also be a chain of filters with syntax as follows:\n",
            "\n",
            "  Ordered in order of decreasing precedence:\n",
            "\n",
            "    * Grouped expression:\n",
            "        \"(\" expr \")\"\n",
            "\n",
            "    * Negated expression:\n",
            "        \"NOT\" expr\n",
            "        \"!\" expr\n",
            "\n",
            "    * Both expr1 and expr2 are true:\n",
            "        expr1 \"AND\" expr2\n",
            "        expr1 expr2\n",
            "      expr2 is not evaluated if expr1 is false.\n",
            "\n",
            "    * One and only one of expr1 and expr2 is true:\n",
            "        expr1 \"XOR\" expr2\n",
            "      Both expressions are evaluated.\n",
            "\n",
            "    * At least one of expr1 and expr2 is true:\n",
            "        expr1 \"OR\" expr2\n",
            "      expr2 is not evaluated if expr1 is true.\n",
            "\n",
            "    * Only return the value of expr2:\n",
            "        expr1 \",\" expr2\n",
            "      Both expressions are evaluated.\n",
            "\n",
            "  Operator names are case-insensitive.\n",
            "\n",
            "  Expressions (unchained):\n",
            "\n",
            "    * Perform a case-sensitive match on file names.\n",
            "        name <glob pattern>\n",
            "      This action is not affected by --full-path.\n",
            "\n",
            "    * Perform a case-insensitive match on file names.\n",
            "        iname <glob pattern>\n",
            "      This action is not affected by --full-path.\n",
            "\n",
            "    * Perform a case-sensitive match on file paths.\n",
            "        path <glob pattern>\n",
            "        regex <regex pattern>\n",
            "      Relative paths always start with \"./\" or \"../\".\n",
            "\n",
            "    * Perform a case-insensitive match on file paths.\n",
            "        ipath <glob pattern>\n",
            "        iregex <regex pattern>\n",
            "      Relative paths always start with \"./\" or \"../\".\n",
            "\n",
            "    * Match specified file types.\n",
            "        type <file type[,file type]...>\n",
            "\n",
            "    * Always true.\n",
            "        true\n",
            "\n",
            "    * Always false.\n",
            "        false\n",
            "\n",
            "    * Always true; print the result followed by a newline.\n",
            "        print\n",
            "\n",
            "    * Always true; print the result followed by a NUL character.\n",
            "        print0\n",
            "\n",
            "    * Always true; do not descend into a directory.\n",
            "        prune\n",
            "      This does not cancel other applied actions.\n",
            "\n",
            "    * Always true; quit searching after applying other actions.\n",
            "        quit\n",
            "      More results can be procuded while this action is accepted.\n",
            "      To make the results predictable, use --sort-path --threads=1.\n",
            "\n",
            "  The head of a predicate is case-insensitive.\n",
            "\n",
            "These predicates are also \"actions\" due to their side effects:\n",
            "\n",
            "  print, print0, prune, quit.\n",
            "\n",
            "If no action is specified in the filter chain, \
             all matched results are printed on the standard output \
             with the line terminator determined by the option --print0.\n",
            "\n",
            "--exec is forbidden while using a filter chain.\n",
            "\n",
            "Please view the man page for example usage. (TODO)"
        )
    );

    help
}
