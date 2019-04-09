# ff — Find Files

[![Build Status](https://travis-ci.org/jakwings/ff-find.svg?branch=master)](https://travis-ci.org/jakwings/ff-find)
[![Version info](https://img.shields.io/crates/v/ff-find.svg)](https://crates.io/crates/ff-find)

ff: Just my own fork of [fd] with many incompatible changes. (**unstable**)

[fd]: https://github.com/sharkdp/fd/tree/7ecb6239504dff9eb9e9359521ece6744ef04f67

## Installation

```
cargo install ff-find
```

## Usage

ff lets you search for files and directories with a [glob pattern](#References).

```bash
ff $HOME '*.txt'
```

More power (and danger) come with the --regex switch to use [regex patterns](#References).

```bash
ff --regex $HOME '\.txt$'
```

Unicode support:

```bash
ff . '?'            # doesn't match filename π
ff --unicode . '?'  # matches filename π

ff --regex . '^.$'            # doesn't match filename π
ff --regex --unicode . '^.$'  # matches filename π
```

This is because the pattern matches byte strings by default.

If you have a disk or partition for backup service, use the --mount flag to
prevent deletion for files on it:

```bash
# skip any directory or files on another disk or partition
ff $HOME .DS_Store --mount --exec rm -v --

# could be faster by working in parallel with xargs
ff $HOME .DS_Store --mount -0 | xargs -0 rm -v --
```

To exclude arbitrary directories or files, try the advanced features:

```bash
# exclamation marks "!" must be escaped for the bash shell
ff / name c++ type directory,symlink \
     \!path '/usr/include/**' \!path '/usr/bin/**'
# ditto
ff / name c++ and type directory,symlink \
     and not path '/usr/include/**' and not path '/usr/bin/**'

# likewise, "iname" means to case-insensitively match file names
ff $HOME iname '*.chm' or iname '*.pdf' or iname '*.epub'
# simpler
ff $HOME iname '*.{chm,pdf,epub}'
```

## Help

```
USAGE:
    ff [OPTIONS] [<DIRECTORY> [PATTERN | FILTER CHAIN]]

OPTIONS:
    -g, --glob
            Match file paths with a glob pattern.
            This is the default behavior.

    -r, --regex
            Match file paths with a regex pattern.

    -u, --unicode
            Turn on Unicode support for search patterns.

            Character classes are not limited to ASCII. Only valid UTF-8 byte
            sequences can be matched by the search pattern.

    -i, --ignore-case
            Perform a case-insensitive search.

    -s, --case-sensitive
            Perform a case-sensitive search.
            This is the default behavior.

    -p, --full-path
            Match the absolute path instead of only the file name or directory
            name.

    -L, --follow
            Follow symlinks and traverse the symlinked directories.

    -M, --mount
            Do not descend into directories on another disk or partition, as a
            symlink or normal directory may lead to a file on another file
            system.

    -0, --print0
            Each search result is terminated with a NUL character instead of a
            newline (LF) when printed.

            This option does not affect --exec.

    -A, --absolute-path
            Relative paths for output are transformed into absolute paths.

            An absolute path may not be the real path due to symlinks.

    -S, --sort-path
            The search results are sorted by pathname before output.

            Sort by lexicographically comparing the byte strings of path
            components. The search depth is also taken into comsideration.

            This option also forces --exec to use a single thread for
            processing.

    -a, --all
            All files and directories are searched.

            By default, files and directories of which the names start with a
            dot "." are ignored in the search.

            Files ignored by patterns in .(git)ignore files are still excluded.

    -I, --no-ignore
            Show search results from files and directories that would otherwise
            be ignored by .(git)ignore files.

    -m, --multiplex
            Multiplex stdin of this program so that every executed command
            shares the same input.

            Interactive input is disabled by caching, even if the commands run
            sequentially.

    -t, --type <filetype>
            Filter the search by type (case-insensitive): [default: any]

                directory or d: directories
                     file or f: regular files
                  symlink or l: symbolic links
               executable or x: executable files

            Executable files are regular files with execute permission bits set
            or are symlinks pointing to the former, which means they are likely
            programs that can be loaded and run on the operating system.

    -d, --max-depth <number>
            Limit the directory traversal to a given depth.

    -c, --color <when>
            Declare when to use color for the pattern match output:

                  auto: use colors for interactive console [default]
                 never: do not use colorized output
                always: always use colorized output

    -j, --threads <number>
            The number of threads to use for searching and command execution.

            0 means [default: number of available CPU cores]

        --max-buffer-time <milliseconds>
            The amount of time (in milliseconds) for the search results to be
            buffered and sorted before streaming.

            This option is mostly a stub for testing purpose.

    -x, --exec <program [argument]... [;]>
            Run the given command for each search result.

            The search result can be represented by a pair of braces {} in the
            command. If the command does not contain any {}, then a {} is
            appended as an argument to the program. A single semicolon ;
            terminates the argument list.

            With --threads=1 commands are run sequentially. If multi-threading
            is enabled and multiplexing is not enabled, commands do not receive
            input from an interactive console.

            If not running with a single thread, each output of the command is
            buffered, reordered (printed to stdout before stderr) and
            synchronized to avoid overlap.

    -v, --verbose
            Show warnings about file permissions, loops caused by symlinks, I/O
            errors, invalid file content, etc.

    -h, --help
            Print help information.
            Use --help to show details and full list of options.
    -V, --version
            Print version information.


ARGS:
    <DIRECTORY>
            The root directory for the search. [optional]
            If omitted, search the current working directory.

    <PATTERN | FILTER CHAIN>...
            A regex or glob pattern for matching files. [optional]
            The default patterns for regex and glob are ^ and * respectively.

            The expression can also be a chain of filters with syntax as
            follows:

              Ordered in order of decreasing precedence:

                * Grouped expression:
                    "(" expr ")"
                * Negated expression:
                    "NOT" expr
                    "!" expr
                * Both expr1 and expr2 are true:
                    expr1 "AND" expr2
                    expr1 expr2
                  expr2 is not evaluated if expr1 is false.
                * One and only one of expr1 and expr2 is true:
                    expr1 "XOR" expr2
                  Both expressions are evaluated.
                * At least one of expr1 and expr2 is true:
                    expr1 "OR" expr2
                  expr2 is not evaluated if expr1 is true.
                * Only return the value of expr2:
                    expr1 "," expr2
                  Both expressions are evaluated.

              Operator names are case-insensitive.

              Expressions (unchained):

                * Perform a case-sensitive match on file names.
                  name <glob pattern>
                * Perform a case-insensitive match on file names.
                  iname <glob pattern>
                * Perform a case-sensitive match on (relative) file paths.
                  path <glob pattern>
                  regex <regex pattern>
                * Perform a case-insensitive match on (relative) file paths.
                  ipath <glob pattern>
                  iregex <regex pattern>
                * Match specified file types.
                  type <file type[,file type]...>
                * Always true.
                  true
                * Always false.
                  false
                * Always true; print the result followed by a newline.
                  print
                * Always true; print the result followed by a NUL character.
                  print0

              The head of a predicate is case-insensitive.

            "print" and "print0" are both predicates and actions.

            If no action is specified in the filter chain, all matched results
            are printed on the standard output with the line terminator
            determined by the option --print0.

            --exec is forbidden while using a filter chain.

            Please view the man page for example usage. (TODO)


NOTE: If the value of environment variable PWD is the path of a symlink pointing
to the current working directory, it is used for resolving the absolute path of
a relative path.
```


## References

*   Glob Syntax: https://docs.rs/ff-find/latest/globset/#syntax
    *   Note: ff uses a variant of [globset][glob] which behaves slightly differently
        for "backslash escape", i.e. `\<char>` drops the `\` and removes special
        effect of a character ANYWHERE.
*   Regex Syntax: https://docs.rs/regex/1.1.2/regex/#syntax

Please note that the nitty-gritty of supported syntax may change in the future.
There are still some todos noted in the source code.

[glob]: https://docs.rs/globset/latest/globset/#syntax


## License

Copyright (c) 2017 ff developers

Copyright (c) 2017 fd developers

All files in the project are licensed under the [Apache License], Version 2.0
or the [MIT License], at your option.

[Apache License]: https://www.apache.org/licenses/LICENSE-2.0
[MIT License]: https://opensource.org/licenses/MIT
