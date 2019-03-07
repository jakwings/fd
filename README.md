# ff — Find Files

[![Build Status](https://travis-ci.org/jakwings/ff-find.svg?branch=master)](https://travis-ci.org/jakwings/ff-find)
[![Version info](https://img.shields.io/crates/v/ff-find.svg)](https://crates.io/crates/ff-find)

ff: Just my own fork of [fd] with many incompatible changes. (**unstable**)

[fd]: https://github.com/sharkdp/fd/tree/7ecb6239504dff9eb9e9359521ece6744ef04f67

## Installation

```
cargo install ff-find
```

or when SIMD acceleration is possible:

```
cargo install --features simd-accel ff-find
```

## Usage

ff lets you search for files and directories with a glob pattern.

```bash
ff $HOME '*.txt'
```

More power (and danger) come with the --regex switch to use regex patterns.

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

## Help

```
USAGE:
    ff [OPTIONS] [<DIRECTORY> [PATTERN]]

OPTIONS:
    -g, --glob
            Match the whole file path with a glob pattern. This is the default
            behavior.

    -r, --regex
            Match the whole file path with a regex pattern. [default: glob]

    -u, --unicode
            Turn on Unicode support for search patterns. Character classes are
            not limited to ASCII. Only valid UTF-8 byte sequences can be matched
            by the search pattern.

    -i, --ignore-case
            Perform a case-insensitive search. This overrides --case-sensitive.

    -s, --case-sensitive
            Perform a case-sensitive search. This is the default behavior.

    -p, --full-path
            Match the absolute path instead of the filename or directory name.

    -L, --follow
            Follow symlinks and traverse the symlinked directories.

    -M, --mount
            Do not descend into directories on other file systems, as a symlink
            or normal directory may lead to a file on another file system.

    -0, --print0
            Each search result is terminated with NUL instead of LF when
            printed.

    -A, --absolute-path
            Relative paths for output are transformed into absolute paths.

    -S, --sort-path
            The search results will be sorted by pathname before output. Sort by
            lexicographically comparing the byte strings of path components (not
            comparing the whole pathnames directly).

    -a, --all
            All files and directories are searched. By default, files and
            directories of which the names start with a dot "." are ignored in
            the search. Files ignored by patterns in .(git)ignore files are
            still excluded.

    -I, --no-ignore
            Show search results from files and directories that would otherwise
            be ignored by .(git)ignore files.

    -m, --multiplex
            Multiplex stdin of this program so that every executed command
            shares the same input. Interactive input is disabled by caching,
            even if the commands run sequentially.

    -t, --type <filetype>
            Filter the search by type: [default: no filter]
                directory or d: directories
                     file or f: regular files
                  symlink or l: symbolic links
               executable or x: executable files

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

    -x, --exec <program [argument]... [;]>
            Run the given command for each search result, which can be
            represented by a pair of braces {} in the command. If the command
            does not contain any {}, then a {} will be appended as an argument
            to the program. A single semicolon ; will terminate the argument
            list.
            With --threads=1 commands will run sequentially. When multi-
            threading is enabled and multiplexing is not enabled, commands
            will not receive input from the terminal. If not running with a
            single thread, each output of the command will be buffered,
            reordered (printed to stdout before stderr) and synchronized to
            avoid overlap.

    -v, --verbose
            Show warnings about file permissions, loops caused by symlinks, I/O
            errors, invalid file content, etc.

    -h, --help
            Prints help information. Use --help to show details and full list of
            options.
    -V, --version
            Prints version information


ARGS:
    <DIRECTORY>
            The directory where the search is rooted. If omitted, search the
            current working directory.

    <PATTERN>
            The search pattern, a regex or glob pattern. [optional]
            The default values for regex and glob are ^ and * respectively.


NOTE: If the value of environment variable PWD is the path of a symlink pointing
to the current working directory, it will be used for resolving the absolute
path of a relative path.
```


## References

*   Glob Syntax: https://docs.rs/globset/0.4.2/globset/#syntax
    *   Note: ff uses a variant of *globset* which behaves slightly differently
        for "backslash escape", i.e. `\<char>` drops the `\` and removes special
        effect of a character ANYWHERE.
*   Regex Syntax: https://docs.rs/regex/1.1.2/regex/#syntax

Please note that the nitty-gritty of supported syntax may change in the future.
There are still some todos noted in the source code.


## License

Copyright (c) 2017 ff developers

Copyright (c) 2017 fd developers

All files in the project are licensed under the [Apache License], Version 2.0
or the [MIT License], at your option.

[Apache License]: https://www.apache.org/licenses/LICENSE-2.0
[MIT License]: https://opensource.org/licenses/MIT
