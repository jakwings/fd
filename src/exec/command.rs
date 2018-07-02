use std::ffi::{OsStr, OsString};
use std::io;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::Path;
use std::process::{Child, Command, Stdio};

#[derive(Clone, Debug, PartialEq)]
pub struct ExecCommand {
    argv: Vec<OsString>,
}

impl ExecCommand {
    pub fn prog(&self) -> &OsString {
        &self.argv[0]
    }

    pub fn args(&self) -> &[OsString] {
        &self.argv[1..]
    }

    pub fn execute(&self, stdin: Stdio, stdout: Stdio, stderr: Stdio) -> io::Result<Child> {
        Command::new(self.prog())
            .args(self.args())
            .stdin(stdin)
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExecTemplate {
    argv: Vec<OsString>,
}

impl ExecTemplate {
    pub fn new(argv: &Vec<&OsStr>) -> ExecTemplate {
        let mut complete = false;
        let mut argv: Vec<_> = argv
            .iter()
            .map(|arg| {
                if !complete && has_stubs(arg) {
                    complete = true;
                }
                arg.to_os_string()
            })
            .collect();

        if !complete {
            argv.push(OsString::from("{}"));
        }

        ExecTemplate { argv }
    }

    pub fn apply(&self, path: &Path) -> ExecCommand {
        ExecCommand {
            argv: self.argv.iter().map(|arg| clear_stubs(arg, path)).collect(),
        }
    }
}

// Check for the existence of "{}".
fn has_stubs(os_str: &OsStr) -> bool {
    let mut iter = os_str.as_bytes().iter();

    loop {
        let (a, b) = (iter.next(), iter.next());
        if a == Some(&b'{') && b == Some(&b'}') {
            return true;
        }
        if b.is_none() {
            break;
        }
    }

    false
}

fn clear_stubs(os_str: &OsStr, path: &Path) -> OsString {
    let mut buffer = OsString::new();
    let mut iter = os_str.as_bytes().iter().peekable();

    while iter.peek().is_some() {
        let mut open = false;

        let bytes = iter
            .by_ref()
            .take_while(|c| {
                open = c == &&b'{';
                !open
            })
            .cloned()
            .collect();
        buffer.push(OsString::from_vec(bytes));

        // TODO: {filename} {basename} {extension} {dirname}
        if open && iter.peek() == Some(&&b'}') {
            buffer.push(path.as_os_str());
            iter.next();
        } else if open {
            // TODO: throw errors for broken and unrecognized {patterns}
            buffer.push("{");
        } // else TODO: an unmatched "}" is "broken" too
    }

    buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! mkv {
        ($($x:expr),*) => (<[_]>::into_vec(Box::new([$(OsStr::new($x)),*])));
        ($($x:expr,)*) => (mkv![$(OsStr::new($x)),*]);
    }

    fn mkv(argv: &[&str]) -> Vec<OsString> {
        argv.to_vec()
            .iter()
            .map(|arg| OsString::from(arg))
            .collect()
    }

    #[test]
    fn template_empty() {
        assert_eq!(ExecTemplate::new(&mkv![]).argv, mkv(&["{}"]));
    }

    #[test]
    fn template_complete() {
        assert_eq!(
            ExecTemplate::new(&mkv!["touch", "{}.mark"]).argv,
            mkv(&["touch", "{}.mark"])
        );
    }

    #[test]
    fn template_apply() {
        let template = ExecTemplate::new(&mkv!["cp", "{}", "{}.bak"]);
        let command = template.apply(&Path::new("foo"));
        assert_eq!(command.argv, mkv(&["cp", "foo", "foo.bak"]));
    }
}
