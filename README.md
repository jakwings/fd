# ff — Find Files

[![Build Status](https://travis-ci.org/jakwings/ff-find.svg?branch=master)](https://travis-ci.org/jakwings/ff-find)
[![Version info](https://img.shields.io/crates/v/ff-find.svg)](https://crates.io/crates/ff-find)

ff: Just my own fork of [fd] with many incompatible changes. (unstable)

[fd]: https://github.com/sharkdp/fd/tree/7ecb6239504dff9eb9e9359521ece6744ef04f67

## Installation

```
cargo install ff-find
```


## Usage

```
ff 0.1.0

USAGE:
    ff [OPTIONS] [DIRECTORY] [PATTERN]

OPTIONS:
    -g, --glob
            Match the whole file path with a glob pattern. [default: use regex
            pattern]

    -r, --regex
            The search pattern is a regex pattern by default. It can match part
            of the file path.

    -u, --unicode
            Turn on Unicode support for regex patterns. Character classes are
            not limited to ASCII. Only valid UTF-8 byte sequences can be matched
            by the search pattern.

    -i, --ignore-case
            Perform a case-insensitive search. This overrides --case-sensitive.

    -s, --case-sensitive
            Perform a case-sensitive search. This overrides --ignore-case.

    -p, --full-path
            Match the absolute path instead of the filename or directory name.

    -L, --follow
            Do not take symlinks as normal files and traverse the symlinked
            directories.

    -0, --print0
            Each search result is terminated with NUL instead of LF when
            printed.

    -A, --absolute-path
            Relative paths for output are transformed into absolute paths.

    -a, --all
            All files and directories are searched. By default, files and
            directories of which the names start with a dot "." are ignored in
            the search.

    -I, --no-ignore
            Show search results from files and directories that would otherwise
            be ignored by .*ignore files.

    -t, --type <filetype>
            Filter the search by type: [default: no filter]
                directory or d: directories
                     file or f: regular files
                  symlink or l: symbolic links
               executable or x: executable regular files

    -d, --max-depth <max-depth>
            Limit the directory traversal to a given depth.

    -c, --color <when>
            Declare when to use color for the pattern match output:
                  auto: use colors for interactive console [default]
                 never: do not use colorized output
                always: always use colorized output

    -j, --threads <number>
            The number of threads to use for searching & command execution. 0
            means [default: number of available CPU cores]

        --max-buffer-time <milliseconds>
            The amount of time for the search results to be buffered and sorted
            before streaming.

    -x, --exec <program [arg]... [;]>
            Run the given command for each search result, which can be
            represented by a pair of braces {} in the command. If the command
            does not contain any {}, then a {} will be appended as an argument
            to the program. A single semicolon ; will terminate the argument
            list.

    -h, --help
            Prints help information. Use --help for more details.

    -V, --version
            Prints version information


ARGS:
    <DIRECTORY>
            The directory where the filesystem search is rooted. If omitted,
            search the current working directory.

    <PATTERN>
            The search pattern, a regular expression or glob string. [optional]
```


## References

*   Regex Syntax: https://docs.rs/regex/0.2.2/regex/#syntax
*   Glob Syntax: https://docs.rs/globset/0.2.1/globset/#syntax

Note that `ff` cannot enable Unicode support for glob patterns. Also, the
nitty-gritty of supported syntax may change in the future. There are still some
todos noted in the source code.


## License

Copyright (c) 2017 ff developers

Copyright (c) 2017 fd developers

Licensed under the [Apache License], Version 2.0 or the [MIT License], at your
option.  All files in the project carrying such notice may not be copied,
modified, or distributed except according to those terms.

[Apache License]: https://www.apache.org/licenses/LICENSE-2.0
[MIT License]: https://opensource.org/licenses/MIT
