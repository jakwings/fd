use std::env;
use std::fs;
use std::io::{self, Write};
use std::os::unix;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process;

extern crate diff;
extern crate tempdir;

use self::tempdir::TempDir;

pub struct TestEnv {
    // temporary working directory
    temp_dir: TempDir,

    // path to the executable
    ff_exe: PathBuf,
}

fn create_working_directory() -> Result<TempDir, io::Error> {
    let temp_dir = TempDir::new("ff-tests")?;

    {
        let root = temp_dir.path();

        fs::create_dir_all(root.join("one/two/three"))?;
        fs::create_dir_all(root.join("one.two"))?;

        let executable = fs::File::create(root.join("a.foo"))?;
        let perms = executable.metadata()?.permissions();
        executable.set_permissions(fs::Permissions::from_mode(perms.mode() | 0o111))?;

        fs::File::create(root.join("one/b.foo"))?;
        fs::File::create(root.join("one/two/c.foo"))?;
        fs::File::create(root.join("one/two/C.Foo2"))?;
        fs::File::create(root.join("one/two/three/d.foo"))?;
        fs::create_dir(root.join("one/two/three/directory_foo"))?;
        fs::File::create(root.join("ignored.foo"))?;
        fs::File::create(root.join(".hidden.foo"))?;
        fs::File::create(root.join("α β"))?;

        unix::fs::symlink(root.join("one/two"), root.join("symlink"))?;
        fs::File::create(root.join("deleted"))?;
        unix::fs::symlink(root.join("deleted"), root.join("symlink2"))?;
        fs::remove_file(root.join("deleted"))?;

        fs::File::create(root.join(".ignore"))?.write_all(b"ignored.foo")?;
    }

    Ok(temp_dir)
}

fn find_ff_exe() -> PathBuf {
    // Tests exe is in target/debug/deps, the *ff* exe is in target/debug
    let dir = env::current_exe()
        .expect("tests executable")
        .parent()
        .expect("tests executable directory")
        .parent()
        .expect("ff executable directory")
        .to_path_buf();

    dir.join("ff")
}

fn format_exit_error(args: &[&str], output: &process::Output) -> String {
    format!(
        "`ff {}` did not exit successfully.\nstdout:\n---\n{}---\nstderr:\n---\n{}---",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn format_output_error(args: &[&str], expected: &str, actual: &str) -> String {
    let diff_text = diff::lines(expected, actual)
        .into_iter()
        .map(|diff| match diff {
            diff::Result::Left(l) => format!("-{}", l),
            diff::Result::Both(l, _) => format!(" {}", l),
            diff::Result::Right(r) => format!("+{}", r),
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        concat!(
            "`ff {}` did not produce the expected output.\n",
            "Showing diff between expected and actual:\n{}\n"
        ),
        args.join(" "),
        diff_text
    )
}

fn normalize_output(s: &str, trim: bool, sort: bool) -> String {
    let text = s.replace('\0', "NULL\n");
    let mut lines = text
        .lines()
        .into_iter()
        .map(|line| if trim { line.trim_start() } else { line })
        .collect::<Vec<_>>();

    if sort {
        lines.sort_unstable();
    }

    lines.join("\n")
}

impl TestEnv {
    pub fn new() -> TestEnv {
        let temp_dir = create_working_directory().expect("working directory");
        let ff_exe = find_ff_exe();

        TestEnv {
            temp_dir: temp_dir,
            ff_exe: ff_exe,
        }
    }

    // Get the root directory for the tests.
    pub fn test_root(&self) -> PathBuf {
        self.temp_dir.path().to_path_buf()
    }

    // Get the root directory of the file system.
    pub fn system_root(&self) -> PathBuf {
        let mut components = self.temp_dir.path().components();
        PathBuf::from(components.next().expect("root directory").as_os_str())
    }

    // Assert that calling *ff* with the specified arguments produces the expected output.
    pub fn assert_output(&self, sort: bool, args: &[&str], expected: &str) {
        self.assert_output_subdirectory(sort, ".", args, expected)
    }

    // Assert that calling *ff* in the specified path under the root working directory,
    // and with the specified arguments produces the expected output.
    pub fn assert_output_subdirectory<P: AsRef<Path>>(
        &self,
        sort: bool,
        path: P,
        args: &[&str],
        expected: &str,
    ) {
        let mut cmd = process::Command::new(&self.ff_exe);
        cmd.current_dir(self.temp_dir.path().join(path));
        cmd.args(args);

        let output = cmd.output().expect("ff output");

        if !output.status.success() {
            panic!(format_exit_error(args, &output));
        }

        let expected = normalize_output(expected, true, sort);
        let actual = normalize_output(&String::from_utf8_lossy(&output.stdout), false, sort);

        if expected != actual {
            panic!(format_output_error(args, &expected, &actual));
        }
    }
}
