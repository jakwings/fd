//! Integration tests for the CLI interface of ff.

extern crate regex;

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

/// Simple tests
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
         ./symlink",
    );

    env.assert_output(
        true,
        &[".", ""],
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
         ./symlink",
    );

    env.assert_output(true, &[".", "a.foo"], "./a.foo");
    env.assert_output(true, &[".", "b.foo"], "./one/b.foo");
    env.assert_output(true, &[".", "d.foo"], "./one/two/three/d.foo");

    env.assert_output(
        true,
        &[".", "foo"],
        "./a.foo
         ./one/b.foo
         ./one/two/c.foo
         ./one/two/three/d.foo
         ./one/two/three/directory_foo",
    );
}

/// Explicit root path
#[test]
fn test_explicit_root_path() {
    let env = TestEnv::new();

    env.assert_output(
        true,
        &["one", "foo"],
        "./one/b.foo
         ./one/two/c.foo
         ./one/two/three/d.foo
         ./one/two/three/directory_foo",
    );

    env.assert_output(
        true,
        &["one/two/three", "foo"],
        "./one/two/three/d.foo
         ./one/two/three/directory_foo",
    );

    env.assert_output_subdirectory(
        true,
        "one/two",
        &["../../", "foo"],
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

/// Regex searches
#[test]
fn test_regex_searches() {
    let env = TestEnv::new();

    env.assert_output(
        true,
        &[".", "[a-c].foo"],
        "./a.foo
         ./one/b.foo
         ./one/two/c.foo",
    );

    env.assert_output(
        true,
        &[".", "[a-c].foo", "--case-sensitive"],
        "./a.foo
         ./one/b.foo
         ./one/two/c.foo",
    );
}

/// Match Unicode string (--unicode)
#[test]
fn test_unicode_aware() {
    let env = TestEnv::new();

    env.assert_output(true, &[".", "\\xCE"], "./α β");
    env.assert_output(true, &["--unicode", ".", "\\xCE"], "");
}

/// Glob searches (--glob)
#[test]
fn test_glob_searches() {
    let env = TestEnv::new();

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
         ./symlink",
    );

    env.assert_output(
        true,
        &["--glob", ".", "*.foo"],
        "./a.foo
         ./one/b.foo
         ./one/two/c.foo
         ./one/two/three/d.foo",
    );

    env.assert_output(
        true,
        &["--glob", "--regex", ".", "[a-c].foo"],
        "./a.foo
         ./one/b.foo
         ./one/two/c.foo",
    );

    env.assert_output(
        true,
        &["--regex", "--glob", ".", "[a-c].foo"],
        "./a.foo
         ./one/b.foo
         ./one/two/c.foo",
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
         ./symlink",
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
}

/// Case sensitivity (--case-sensitive)
#[test]
fn test_case_sensitive() {
    let env = TestEnv::new();

    env.assert_output(true, &[".", "c.foo", "--case-sensitive"], "./one/two/c.foo");

    env.assert_output(
        true,
        &[".", "C.Foo", "--case-sensitive"],
        "./one/two/C.Foo2",
    );

    env.assert_output(
        true,
        &[".", "C.Foo", "--ignore-case", "--case-sensitive"],
        "./one/two/C.Foo2",
    );
}

/// Case insensitivity (--ignore-case)
#[test]
fn test_case_insensitive() {
    let env = TestEnv::new();

    env.assert_output(
        true,
        &[".", "C.Foo", "--ignore-case"],
        "./one/two/C.Foo2
         ./one/two/c.foo",
    );

    env.assert_output(
        true,
        &[".", "C.Foo", "--case-sensitive", "--ignore-case"],
        "./one/two/C.Foo2
         ./one/two/c.foo",
    );
}

/// Full path search (--full-path)
#[test]
fn test_full_path() {
    let env = TestEnv::new();

    let root = env.system_root();
    let prefix = escape(&root.to_string_lossy());

    env.assert_output(
        true,
        &[
            ".",
            &format!("^{prefix}.*three.*foo$", prefix = prefix),
            "--full-path",
        ],
        "./one/two/three/d.foo
         ./one/two/three/directory_foo",
    );
}

/// Hidden files (--all)
#[test]
fn test_hidden() {
    let env = TestEnv::new();

    env.assert_output(
        true,
        &[".", "foo", "--all"],
        "./.hidden.foo
         ./a.foo
         ./one/b.foo
         ./one/two/c.foo
         ./one/two/three/d.foo
         ./one/two/three/directory_foo",
    );

    env.assert_output(
        true,
        &[".", "foo", "--all", "--no-ignore"],
        "./.hidden.foo
         ./a.foo
         ./ignored.foo
         ./one/b.foo
         ./one/two/c.foo
         ./one/two/three/d.foo
         ./one/two/three/directory_foo",
    );
}

/// Ignored files (--no-ignore)
#[test]
fn test_no_ignore() {
    let env = TestEnv::new();

    env.assert_output(
        true,
        &[".", "foo", "--no-ignore"],
        "./a.foo
         ./ignored.foo
         ./one/b.foo
         ./one/two/c.foo
         ./one/two/three/d.foo
         ./one/two/three/directory_foo",
    );
}

/// Symlinks (--follow)
#[test]
fn test_follow() {
    let env = TestEnv::new();

    env.assert_output(
        true,
        &[".", "c.foo", "--follow"],
        "./one/two/c.foo
         ./symlink/c.foo",
    );
}

/// Null separator (--print0)
#[test]
fn test_print0() {
    let env = TestEnv::new();

    env.assert_output(
        true,
        &[".", "foo", "--print0"],
        "./a.fooNULL
         ./one/b.fooNULL
         ./one/two/c.fooNULL
         ./one/two/three/d.fooNULL
         ./one/two/three/directory_fooNULL",
    );
}

/// Maximum depth (--max-depth)
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
         ./symlink",
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
         ./symlink",
    );

    env.assert_output(
        true,
        &["--max-depth", "1"],
        "./a.foo
         ./α β
         ./one
         ./one.two
         ./symlink",
    );

    env.assert_output(true, &["--max-depth", "0"], "");
}

/// Absolute paths (--absolute-path)
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
             {abs_path}/symlink",
            abs_path = abs_path
        ),
    );

    env.assert_output(
        true,
        &[".", "foo", "--absolute-path"],
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
        &[&abs_path, "foo"],
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

/// Sort paths (--sort-path)
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
         ./α β",
    );
}

/// File type filter (--type)
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

    env.assert_output(true, &["--type", "l"], "./symlink");

    env.assert_output(true, &["--type", "x"], "./a.foo");
}

/// Symlinks misc
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
             {dir}/symlink",
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

/// Shell script execution (--exec)
#[test]
fn test_exec() {
    let env = TestEnv::new();

    let abs_path = get_test_root(&env);

    env.assert_output(
        true,
        &[".", "foo", "--absolute-path", "--exec", "printf", "%s\\n"],
        &format!(
            "{abs_path}/a.foo
             {abs_path}/one/b.foo
             {abs_path}/one/two/c.foo
             {abs_path}/one/two/three/d.foo
             {abs_path}/one/two/three/directory_foo",
            abs_path = &abs_path
        ),
    );

    env.assert_output(
        true,
        &[
            ".",
            "foo",
            "--exec",
            "printf",
            "%s\\n",
            ";",
            "--absolute-path",
        ],
        &format!(
            "{abs_path}/a.foo
             {abs_path}/one/b.foo
             {abs_path}/one/two/c.foo
             {abs_path}/one/two/three/d.foo
             {abs_path}/one/two/three/directory_foo",
            abs_path = &abs_path
        ),
    );

    env.assert_output(
        true,
        &[".", "foo", "--exec", "printf", "%s\\n", "{}"],
        "./a.foo
         ./one/b.foo
         ./one/two/c.foo
         ./one/two/three/d.foo
         ./one/two/three/directory_foo",
    );

    env.assert_output(
        true,
        &[".", "α β", "--exec", "printf", "%s.%s\\n"],
        "./α β.",
    );
}
