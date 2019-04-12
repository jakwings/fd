//! Integration tests for the CLI interface of ff.

extern crate regex;

// TODO: Reorganize the test suit.
mod testenv;

use regex::escape;

use self::testenv::TestEnv;

fn get_test_root(env: &TestEnv) -> String {
    env.test_root()
        .canonicalize()
        .expect("real path")
        .to_str()
        .expect("utf-8 string")
        .to_string()
}

#[test]
fn test_simple() {
    let env = TestEnv::new();

    env.assert_output(
        true,
        &[],
        "./a.foo
         ./α β
         ./one
         ./one.two
         ./one/b.foo
         ./one/two
         ./one/two/C.Foo2
         ./one/two/c.foo
         ./one/two/three
         ./one/two/three/d.foo
         ./one/two/three/directory_foo
         ./symlink
         ./symlink2",
    );

    env.assert_output(
        true,
        &["./"],
        "./a.foo
         ./α β
         ./one
         ./one.two
         ./one/b.foo
         ./one/two
         ./one/two/C.Foo2
         ./one/two/c.foo
         ./one/two/three
         ./one/two/three/d.foo
         ./one/two/three/directory_foo
         ./symlink
         ./symlink2",
    );

    env.assert_output(true, &["a.foo"], "./a.foo");
}

#[test]
fn test_glob_searches() {
    let env = TestEnv::new();

    env.assert_output(true, &[".", ""], "");

    env.assert_output(
        true,
        &["--glob"],
        "./a.foo
         ./α β
         ./one
         ./one.two
         ./one/b.foo
         ./one/two
         ./one/two/C.Foo2
         ./one/two/c.foo
         ./one/two/three
         ./one/two/three/d.foo
         ./one/two/three/directory_foo
         ./symlink
         ./symlink2",
    );

    env.assert_output(
        true,
        &["--glob", ".", "*"],
        "./a.foo
         ./α β
         ./one
         ./one.two
         ./one/b.foo
         ./one/two
         ./one/two/C.Foo2
         ./one/two/c.foo
         ./one/two/three
         ./one/two/three/d.foo
         ./one/two/three/directory_foo
         ./symlink
         ./symlink2",
    );

    env.assert_output(
        true,
        &["--glob", ".", "*.foo"],
        "./a.foo
         ./one/b.foo
         ./one/two/c.foo
         ./one/two/three/d.foo",
    );

    env.assert_output(true, &["--glob", ".", "*", "--full-path"], "");

    env.assert_output(
        true,
        &["--glob", ".", "**", "--full-path"],
        "./a.foo
         ./α β
         ./one
         ./one.two
         ./one/b.foo
         ./one/two
         ./one/two/C.Foo2
         ./one/two/c.foo
         ./one/two/three
         ./one/two/three/d.foo
         ./one/two/three/directory_foo
         ./symlink
         ./symlink2",
    );

    env.assert_output(
        true,
        &["--glob", ".", "**/*.foo", "--full-path"],
        "./a.foo
         ./one/b.foo
         ./one/two/c.foo
         ./one/two/three/d.foo",
    );

    env.assert_output(
        true,
        &["--glob", ".", "*/**/*.foo", "--full-path"],
        "./a.foo
         ./one/b.foo
         ./one/two/c.foo
         ./one/two/three/d.foo",
    );

    env.assert_output(
        true,
        &["--glob", ".", "**/**/*.foo", "--full-path"],
        "./a.foo
         ./one/b.foo
         ./one/two/c.foo
         ./one/two/three/d.foo",
    );

    env.assert_output(
        true,
        &["--regex", "--glob", ".", "[a-c].foo"],
        "./a.foo
         ./one/b.foo
         ./one/two/c.foo",
    );
}

#[test]
fn test_regex_searches() {
    let env = TestEnv::new();

    env.assert_output(
        true,
        &["--regex"],
        "./a.foo
         ./α β
         ./one
         ./one.two
         ./one/b.foo
         ./one/two
         ./one/two/C.Foo2
         ./one/two/c.foo
         ./one/two/three
         ./one/two/three/d.foo
         ./one/two/three/directory_foo
         ./symlink
         ./symlink2",
    );

    env.assert_output(
        true,
        &["--regex", ".", ""],
        "./a.foo
         ./α β
         ./one
         ./one.two
         ./one/b.foo
         ./one/two
         ./one/two/C.Foo2
         ./one/two/c.foo
         ./one/two/three
         ./one/two/three/d.foo
         ./one/two/three/directory_foo
         ./symlink
         ./symlink2",
    );

    env.assert_output(true, &["--regex", ".", "a.foo"], "./a.foo");
    env.assert_output(true, &["--regex", ".", "b.foo"], "./one/b.foo");
    env.assert_output(true, &["--regex", ".", "d.foo"], "./one/two/three/d.foo");

    env.assert_output(
        true,
        &["--regex", ".", "foo"],
        "./a.foo
         ./one/b.foo
         ./one/two/c.foo
         ./one/two/three/d.foo
         ./one/two/three/directory_foo",
    );

    env.assert_output(
        true,
        &["--glob", "--regex", ".", "[a-c].foo"],
        "./a.foo
         ./one/b.foo
         ./one/two/c.foo",
    );
}

#[test]
fn test_explicit_root_path() {
    let env = TestEnv::new();

    env.assert_output(
        true,
        &["--regex", "one", "foo"],
        "./one/b.foo
         ./one/two/c.foo
         ./one/two/three/d.foo
         ./one/two/three/directory_foo",
    );

    env.assert_output(
        true,
        &["--regex", "one/two/three", "foo"],
        "./one/two/three/d.foo
         ./one/two/three/directory_foo",
    );

    env.assert_output_subdirectory(
        true,
        "one/two",
        &["--regex", "../../", "foo"],
        "../../a.foo
         ../../one/b.foo
         ../../one/two/c.foo
         ../../one/two/three/d.foo
         ../../one/two/three/directory_foo",
    );

    env.assert_output_subdirectory(
        true,
        "one/two/three",
        &[".."],
        "../C.Foo2
         ../c.foo
         ../three
         ../three/d.foo
         ../three/directory_foo",
    );
}

#[test]
fn test_unicode_aware() {
    let env = TestEnv::new();

    env.assert_output(true, &["--glob", ".", "α??"], "");

    env.assert_output(true, &["--glob", "--unicode", ".", "α??"], "./α β");

    env.assert_output(true, &["--regex", ".", "^α"], "./α β");

    //env.assert_output(true, &["--regex", ".", "^(?u:α)"], "./α β");

    env.assert_output(true, &["--regex", ".", "^\\xCE"], "./α β");

    env.assert_output(true, &["--regex", "--unicode", ".", "^\\xCE"], "");

    //env.assert_output(true, &["--regex", "--unicode", ".", "^\\xCE\\xB1"], "./α β");

    env.assert_output(
        true,
        &["--regex", "--unicode", ".", "^[α β]{3}$"],
        "./α β",
    );

    //env.assert_output(true, &["--regex", "--unicode", ".", "^(?-u:α)"], "./α β");

    env.assert_output(
        true,
        &["--regex", "--unicode", ".", "^(?-u:\\xCE\\xB1)"],
        "./α β",
    );
}

#[test]
fn test_case_sensitive() {
    let env = TestEnv::new();

    env.assert_output(
        true,
        &["--regex", ".", "c.foo", "--case-sensitive"],
        "./one/two/c.foo",
    );

    env.assert_output(
        true,
        &["--regex", ".", "C.Foo", "--case-sensitive"],
        "./one/two/C.Foo2",
    );

    env.assert_output(
        true,
        &["--regex", ".", "C.Foo", "--ignore-case", "--case-sensitive"],
        "./one/two/C.Foo2",
    );
}

#[test]
fn test_case_insensitive() {
    let env = TestEnv::new();

    env.assert_output(
        true,
        &["--regex", ".", "C.Foo", "--ignore-case"],
        "./one/two/C.Foo2
         ./one/two/c.foo",
    );

    env.assert_output(
        true,
        &["--regex", ".", "C.Foo", "--case-sensitive", "--ignore-case"],
        "./one/two/C.Foo2
         ./one/two/c.foo",
    );
}

#[test]
fn test_full_path() {
    let env = TestEnv::new();

    let root = env.system_root();
    let prefix = escape(&root.to_string_lossy());

    env.assert_output(
        true,
        &[
            "--regex",
            ".",
            &format!("^{prefix}.*three.*foo$", prefix = prefix),
            "--full-path",
        ],
        "./one/two/three/d.foo
         ./one/two/three/directory_foo",
    );
}

#[test]
fn test_hidden() {
    let env = TestEnv::new();

    env.assert_output(
        true,
        &["--regex", ".", "foo", "--all"],
        "./.hidden.foo
         ./a.foo
         ./one/b.foo
         ./one/two/c.foo
         ./one/two/three/d.foo
         ./one/two/three/directory_foo",
    );

    env.assert_output(
        true,
        &["--regex", ".", "foo", "--all", "--no-ignore"],
        "./.hidden.foo
         ./a.foo
         ./ignored.foo
         ./one/b.foo
         ./one/two/c.foo
         ./one/two/three/d.foo
         ./one/two/three/directory_foo",
    );
}

#[test]
fn test_no_ignore() {
    let env = TestEnv::new();

    env.assert_output(
        true,
        &["--regex", ".", "foo", "--no-ignore"],
        "./a.foo
         ./ignored.foo
         ./one/b.foo
         ./one/two/c.foo
         ./one/two/three/d.foo
         ./one/two/three/directory_foo",
    );
}

#[test]
fn test_follow() {
    let env = TestEnv::new();

    env.assert_output(
        true,
        &["--follow"],
        "./a.foo
         ./α β
         ./one
         ./one.two
         ./one/b.foo
         ./one/two
         ./one/two/C.Foo2
         ./one/two/c.foo
         ./one/two/three
         ./one/two/three/d.foo
         ./one/two/three/directory_foo
         ./symlink
         ./symlink/C.Foo2
         ./symlink/c.foo
         ./symlink/three
         ./symlink/three/d.foo
         ./symlink/three/directory_foo
         ./symlink2",
    );

    env.assert_output(
        true,
        &["--regex", ".", "c.foo", "--follow"],
        "./one/two/c.foo
         ./symlink/c.foo",
    );
}

#[test]
fn test_print0() {
    let env = TestEnv::new();

    env.assert_output(
        true,
        &["--regex", ".", "foo", "--print0"],
        "./a.fooNULL
         ./one/b.fooNULL
         ./one/two/c.fooNULL
         ./one/two/three/d.fooNULL
         ./one/two/three/directory_fooNULL",
    );
}

#[test]
fn test_max_depth() {
    let env = TestEnv::new();

    env.assert_output(
        true,
        &["--max-depth", "3"],
        "./a.foo
         ./α β
         ./one
         ./one.two
         ./one/b.foo
         ./one/two
         ./one/two/C.Foo2
         ./one/two/c.foo
         ./one/two/three
         ./symlink
         ./symlink2",
    );

    env.assert_output(
        true,
        &["--max-depth", "2"],
        "./a.foo
         ./α β
         ./one
         ./one.two
         ./one/b.foo
         ./one/two
         ./symlink
         ./symlink2",
    );

    env.assert_output(
        true,
        &["--max-depth", "1"],
        "./a.foo
         ./α β
         ./one
         ./one.two
         ./symlink
         ./symlink2",
    );

    env.assert_output(true, &["--max-depth", "0"], "");
}

#[test]
fn test_absolute_path() {
    let env = TestEnv::new();

    let abs_path = get_test_root(&env);

    env.assert_output(
        true,
        &["--absolute-path"],
        &format!(
            "{abs_path}/a.foo
             {abs_path}/α β
             {abs_path}/one
             {abs_path}/one.two
             {abs_path}/one/b.foo
             {abs_path}/one/two
             {abs_path}/one/two/C.Foo2
             {abs_path}/one/two/c.foo
             {abs_path}/one/two/three
             {abs_path}/one/two/three/d.foo
             {abs_path}/one/two/three/directory_foo
             {abs_path}/symlink
             {abs_path}/symlink2",
            abs_path = abs_path
        ),
    );

    env.assert_output(
        true,
        &["--regex", ".", "foo", "--absolute-path"],
        &format!(
            "{abs_path}/a.foo
             {abs_path}/one/b.foo
             {abs_path}/one/two/c.foo
             {abs_path}/one/two/three/d.foo
             {abs_path}/one/two/three/directory_foo",
            abs_path = abs_path
        ),
    );

    env.assert_output(
        true,
        &["--regex", &abs_path, "foo"],
        &format!(
            "{abs_path}/a.foo
             {abs_path}/one/b.foo
             {abs_path}/one/two/c.foo
             {abs_path}/one/two/three/d.foo
             {abs_path}/one/two/three/directory_foo",
            abs_path = abs_path
        ),
    );
}

#[test]
fn test_sort_path() {
    let env = TestEnv::new();

    env.assert_output(
        false,
        &["--sort-path"],
        "./a.foo
         ./one
         ./one/b.foo
         ./one/two
         ./one/two/C.Foo2
         ./one/two/c.foo
         ./one/two/three
         ./one/two/three/d.foo
         ./one/two/three/directory_foo
         ./one.two
         ./symlink
         ./symlink2
         ./α β",
    );
}

#[test]
fn test_type() {
    let env = TestEnv::new();

    env.assert_output(
        true,
        &["--type", "f"],
        "./a.foo
         ./α β
         ./one/b.foo
         ./one/two/C.Foo2
         ./one/two/c.foo
         ./one/two/three/d.foo",
    );

    env.assert_output(
        true,
        &["--type", "d"],
        "./one
         ./one.two
         ./one/two
         ./one/two/three
         ./one/two/three/directory_foo",
    );

    env.assert_output(
        true,
        &["--type", "l"],
        "./symlink
         ./symlink2",
    );

    env.assert_output(true, &["--type", "x"], "./a.foo");

    env.assert_output(
        true,
        &["--type", "x,l"],
        "./a.foo
         ./symlink
         ./symlink2",
    );
}

#[test]
fn test_symlink() {
    let env = TestEnv::new();

    let abs_path = get_test_root(&env);

    // From: http://pubs.opengroup.org/onlinepubs/9699919799/functions/getcwd.html
    // The getcwd() function shall place an absolute pathname of the current working directory in
    // the array pointed to by buf, and return buf. The pathname shall contain no components that
    // are dot or dot-dot, or are symbolic links.
    //
    // Key points:
    // 1. The path of the current working directory of a Unix process cannot contain symlinks.
    // 2. The path of the current working directory of a Windows process can contain symlinks.
    //
    // More:
    // 1. On Windows, symlinks are resolved after the ".." component.
    // 2. On Unix, symlinks are resolved immediately as encountered.

    let parent_parent = if cfg!(windows) { ".." } else { "../.." };
    env.assert_output_subdirectory(
        true,
        "symlink",
        &[parent_parent],
        &format!(
            "{dir}/a.foo
             {dir}/α β
             {dir}/one
             {dir}/one.two
             {dir}/one/b.foo
             {dir}/one/two
             {dir}/one/two/C.Foo2
             {dir}/one/two/c.foo
             {dir}/one/two/three
             {dir}/one/two/three/d.foo
             {dir}/one/two/three/directory_foo
             {dir}/symlink
             {dir}/symlink2",
            dir = parent_parent
        ),
    );

    env.assert_output_subdirectory(
        true,
        "symlink",
        &["--absolute-path"],
        &format!(
            "{abs_path}/{dir}/C.Foo2
             {abs_path}/{dir}/c.foo
             {abs_path}/{dir}/three
             {abs_path}/{dir}/three/d.foo
             {abs_path}/{dir}/three/directory_foo",
            dir = if cfg!(windows) { "symlink" } else { "one/two" },
            abs_path = abs_path
        ),
    );

    env.assert_output(
        true,
        &[&format!("{abs_path}/symlink", abs_path = abs_path)],
        &format!(
            "{abs_path}/symlink/C.Foo2
             {abs_path}/symlink/c.foo
             {abs_path}/symlink/three
             {abs_path}/symlink/three/d.foo
             {abs_path}/symlink/three/directory_foo",
            abs_path = abs_path
        ),
    );

    let root = env.system_root();
    let prefix = escape(&root.to_string_lossy());

    env.assert_output_subdirectory(
        true,
        "symlink",
        &[
            "--regex",
            ".",
            &format!("^{prefix}.*three", prefix = prefix),
            "--full-path",
            "--absolute-path",
        ],
        &format!(
            "{abs_path}/{dir}/three
             {abs_path}/{dir}/three/d.foo
             {abs_path}/{dir}/three/directory_foo",
            dir = if cfg!(windows) { "symlink" } else { "one/two" },
            abs_path = abs_path
        ),
    );

    env.assert_output(
        true,
        &[
            "--regex",
            &format!("{abs_path}/symlink", abs_path = abs_path),
            &format!("^{prefix}.*symlink.*three", prefix = prefix),
            "--full-path",
        ],
        &format!(
            "{abs_path}/symlink/three
             {abs_path}/symlink/three/d.foo
             {abs_path}/symlink/three/directory_foo",
            abs_path = abs_path
        ),
    );
}

#[test]
fn test_exec() {
    let env = TestEnv::new();

    let abs_path = get_test_root(&env);

    env.assert_output(
        true,
        &[
            "--regex",
            ".",
            "foo",
            "--absolute-path",
            "--threads=1",
            "--exec",
            "printf",
            ": %s\\n",
        ],
        &format!(
            ": {abs_path}/a.foo
             : {abs_path}/one/b.foo
             : {abs_path}/one/two/c.foo
             : {abs_path}/one/two/three/d.foo
             : {abs_path}/one/two/three/directory_foo",
            abs_path = &abs_path
        ),
    );

    env.assert_output(
        true,
        &[
            "--regex",
            ".",
            "foo",
            "--threads=1",
            "--exec",
            "printf",
            ": %s\\n",
            ";",
            "--absolute-path",
        ],
        &format!(
            ": {abs_path}/a.foo
             : {abs_path}/one/b.foo
             : {abs_path}/one/two/c.foo
             : {abs_path}/one/two/three/d.foo
             : {abs_path}/one/two/three/directory_foo",
            abs_path = &abs_path
        ),
    );

    env.assert_output(
        true,
        &[
            "--regex",
            ".",
            "foo",
            "--threads=1",
            "--exec",
            "printf",
            ": %s\\n",
            "{}",
        ],
        ": ./a.foo
         : ./one/b.foo
         : ./one/two/c.foo
         : ./one/two/three/d.foo
         : ./one/two/three/directory_foo",
    );

    env.assert_output(
        true,
        &[
            "--regex",
            ".",
            "α β",
            "--threads=1",
            "--exec",
            "printf",
            ": %s.%s\\n",
        ],
        ": ./α β.",
    );

    env.assert_output(
        false,
        &["--threads=4", "--sort-path", "--exec", "printf", ": %s\\n"],
        ": ./a.foo
         : ./one
         : ./one/b.foo
         : ./one/two
         : ./one/two/C.Foo2
         : ./one/two/c.foo
         : ./one/two/three
         : ./one/two/three/d.foo
         : ./one/two/three/directory_foo
         : ./one.two
         : ./symlink
         : ./symlink2
         : ./α β",
    );

    // TODO: Test isatty(stdin)
    // TODO: Test multiplexer for single-thread and multi-thread execution.
}

#[test]
fn test_include_dirs() {
    let env = TestEnv::new();

    env.assert_output(
        true,
        &["--include", "."],
        "./a.foo
         ./a.foo
         ./α β
         ./α β
         ./one
         ./one
         ./one.two
         ./one.two
         ./one/b.foo
         ./one/b.foo
         ./one/two
         ./one/two
         ./one/two/C.Foo2
         ./one/two/C.Foo2
         ./one/two/c.foo
         ./one/two/c.foo
         ./one/two/three
         ./one/two/three
         ./one/two/three/d.foo
         ./one/two/three/d.foo
         ./one/two/three/directory_foo
         ./one/two/three/directory_foo
         ./symlink
         ./symlink
         ./symlink2
         ./symlink2",
    );

    let abs_path = get_test_root(&env);

    env.assert_output(
        true,
        &["--include", &abs_path],
        &format!(
            "./a.foo
             ./α β
             ./one
             ./one.two
             ./one/b.foo
             ./one/two
             ./one/two/C.Foo2
             ./one/two/c.foo
             ./one/two/three
             ./one/two/three/d.foo
             ./one/two/three/directory_foo
             ./symlink
             ./symlink2
             {abs_path}/a.foo
             {abs_path}/α β
             {abs_path}/one
             {abs_path}/one.two
             {abs_path}/one/b.foo
             {abs_path}/one/two
             {abs_path}/one/two/C.Foo2
             {abs_path}/one/two/c.foo
             {abs_path}/one/two/three
             {abs_path}/one/two/three/d.foo
             {abs_path}/one/two/three/directory_foo
             {abs_path}/symlink
             {abs_path}/symlink2",
            abs_path = abs_path
        ),
    );

    env.assert_output(
        true,
        &[&abs_path, "--include", "."],
        &format!(
            "./a.foo
             ./α β
             ./one
             ./one.two
             ./one/b.foo
             ./one/two
             ./one/two/C.Foo2
             ./one/two/c.foo
             ./one/two/three
             ./one/two/three/d.foo
             ./one/two/three/directory_foo
             ./symlink
             ./symlink2
             {abs_path}/a.foo
             {abs_path}/α β
             {abs_path}/one
             {abs_path}/one.two
             {abs_path}/one/b.foo
             {abs_path}/one/two
             {abs_path}/one/two/C.Foo2
             {abs_path}/one/two/c.foo
             {abs_path}/one/two/three
             {abs_path}/one/two/three/d.foo
             {abs_path}/one/two/three/directory_foo
             {abs_path}/symlink
             {abs_path}/symlink2",
            abs_path = abs_path
        ),
    );
}

#[test]
fn test_exclude_dirs() {
    let env = TestEnv::new();

    env.assert_output(
        true,
        &["--exclude", "one/two"],
        "./a.foo
         ./α β
         ./one
         ./one.two
         ./one/b.foo
         ./symlink
         ./symlink2",
    );

    let abs_path = get_test_root(&env);

    env.assert_output(
        true,
        &[&abs_path, "--exclude", "one/two"],
        &format!(
            "{abs_path}/a.foo
             {abs_path}/α β
             {abs_path}/one
             {abs_path}/one.two
             {abs_path}/one/b.foo
             {abs_path}/one/two
             {abs_path}/one/two/C.Foo2
             {abs_path}/one/two/c.foo
             {abs_path}/one/two/three
             {abs_path}/one/two/three/d.foo
             {abs_path}/one/two/three/directory_foo
             {abs_path}/symlink
             {abs_path}/symlink2",
            abs_path = abs_path
        ),
    );

    env.assert_output(
        true,
        &["--exclude", &format!("{}/one/two", abs_path)],
        "./a.foo
         ./α β
         ./one
         ./one.two
         ./one/b.foo
         ./one/two
         ./one/two/C.Foo2
         ./one/two/c.foo
         ./one/two/three
         ./one/two/three/d.foo
         ./one/two/three/directory_foo
         ./symlink
         ./symlink2",
    );
}

#[test]
fn test_filter_chain() {
    let env = TestEnv::new();

    env.assert_output(true, &[".", "not", "true"], "");

    env.assert_output(
        true,
        &[".", "not", "false"],
        "./a.foo
         ./α β
         ./one
         ./one.two
         ./one/b.foo
         ./one/two
         ./one/two/C.Foo2
         ./one/two/c.foo
         ./one/two/three
         ./one/two/three/d.foo
         ./one/two/three/directory_foo
         ./symlink
         ./symlink2",
    );

    env.assert_output(
        true,
        &["--regex", ".", "name", "*"],
        "./a.foo
         ./α β
         ./one
         ./one.two
         ./one/b.foo
         ./one/two
         ./one/two/C.Foo2
         ./one/two/c.foo
         ./one/two/three
         ./one/two/three/d.foo
         ./one/two/three/directory_foo
         ./symlink
         ./symlink2",
    );

    env.assert_output(true, &[".", "name", "./**"], "");

    env.assert_output(true, &[".", "name", "/**"], "");

    env.assert_output(true, &[".", "name", "./**", "--full-path"], "");

    env.assert_output(true, &[".", "name", "/**", "--full-path"], "");

    env.assert_output(
        true,
        &["--regex", ".", "path", "./**"],
        "./a.foo
         ./α β
         ./one
         ./one.two
         ./one/b.foo
         ./one/two
         ./one/two/C.Foo2
         ./one/two/c.foo
         ./one/two/three
         ./one/two/three/d.foo
         ./one/two/three/directory_foo
         ./symlink
         ./symlink2",
    );

    env.assert_output(
        true,
        &["--regex", ".", "path", "/**", "--full-path"],
        "./a.foo
         ./α β
         ./one
         ./one.two
         ./one/b.foo
         ./one/two
         ./one/two/C.Foo2
         ./one/two/c.foo
         ./one/two/three
         ./one/two/three/d.foo
         ./one/two/three/directory_foo
         ./symlink
         ./symlink2",
    );

    env.assert_output(
        true,
        &["--glob", ".", "regex", "^./"],
        "./a.foo
         ./α β
         ./one
         ./one.two
         ./one/b.foo
         ./one/two
         ./one/two/C.Foo2
         ./one/two/c.foo
         ./one/two/three
         ./one/two/three/d.foo
         ./one/two/three/directory_foo
         ./symlink
         ./symlink2",
    );

    env.assert_output(
        true,
        &["--glob", ".", "regex", "^/", "--full-path"],
        "./a.foo
         ./α β
         ./one
         ./one.two
         ./one/b.foo
         ./one/two
         ./one/two/C.Foo2
         ./one/two/c.foo
         ./one/two/three
         ./one/two/three/d.foo
         ./one/two/three/directory_foo
         ./symlink
         ./symlink2",
    );

    env.assert_output(
        true,
        &[".", "name", "c.foo", "--ignore-case"],
        "./one/two/c.foo",
    );

    env.assert_output(
        true,
        &[".", "path", "**/c.foo", "--ignore-case"],
        "./one/two/c.foo",
    );

    env.assert_output(
        true,
        &[".", "regex", "/c.foo$", "--ignore-case"],
        "./one/two/c.foo",
    );

    env.assert_output(
        true,
        &[".", "iname", "C.Foo", "--case-sensitive"],
        "./one/two/c.foo",
    );

    env.assert_output(
        true,
        &[".", "ipath", "**/C.Foo", "--case-sensitive"],
        "./one/two/c.foo",
    );

    env.assert_output(
        true,
        &[".", "iregex", "/C.Foo$", "--case-sensitive"],
        "./one/two/c.foo",
    );

    env.assert_output(true, &[".", "false", "and", "print", "or", "name", "*"], "");

    env.assert_output(
        true,
        &[".", "name", "*foo", "and", "print", "--print0"],
        "./a.foo
         ./one/b.foo
         ./one/two/c.foo
         ./one/two/three/d.foo
         ./one/two/three/directory_foo",
    );

    env.assert_output(
        true,
        &[".", "name", "*foo", "and", "print0"],
        "./a.fooNULL
         ./one/b.fooNULL
         ./one/two/c.fooNULL
         ./one/two/three/d.fooNULL
         ./one/two/three/directory_fooNULL",
    );

    env.assert_output(
        true,
        &[".", "name", "*foo", "and", "(", "print", ",", "print0", ")"],
        "./a.foo
         ./a.fooNULL
         ./one/b.foo
         ./one/b.fooNULL
         ./one/two/c.foo
         ./one/two/c.fooNULL
         ./one/two/three/d.foo
         ./one/two/three/d.fooNULL
         ./one/two/three/directory_foo
         ./one/two/three/directory_fooNULL",
    );

    env.assert_output(
        true,
        &[".", "type", "f,d", "and", "!", "type", "x"],
        "./α β
         ./one
         ./one/b.foo
         ./one.two
         ./one/two
         ./one/two/C.Foo2
         ./one/two/c.foo
         ./one/two/three
         ./one/two/three/d.foo
         ./one/two/three/directory_foo",
    );

    env.assert_output(
        true,
        &[".", "name", "one", "prune", "or", "print"],
        "./a.foo
         ./α β
         ./one.two
         ./symlink
         ./symlink2",
    );

    env.assert_output(
        true,
        &[".", "name", "one", "prune", "print", "or", "print"],
        "./a.foo
         ./α β
         ./one
         ./one.two
         ./symlink
         ./symlink2",
    );

    env.assert_output(
        true,
        &[".", "name", "one", "prune", "print", "or", "print", "-Eone"],
        "./a.foo
         ./α β
         ./one.two
         ./symlink
         ./symlink2",
    );

    // hard to test due to multi-threading
    env.assert_output(true, &[".", "quit", "or", "print"], "");

    env.assert_output(
        true,
        &["-Sj1", ".", "name", "one", "quit", "or", "print"],
        "./a.foo",
    );
}
