use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;

use super::ticket::ExecTicket;

#[derive(Clone, Debug, PartialEq)]
pub struct ExecCommand {
    pub prog: OsString,
    pub args: Vec<OsString>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExecTemplate {
    argv: Vec<OsString>,
}

impl ExecTemplate {
    pub fn new(argv: &Vec<&OsStr>) -> ExecTemplate {
        let mut complete = false;
        let mut argv: Vec<_> = argv.iter()
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

    pub fn generate(
        &self,
        path: &Path,
        out_lock: Arc<Mutex<()>>,
        quitting: Arc<AtomicBool>,
    ) -> ExecTicket {
        let command = self.apply(path);

        ExecTicket::new(command, out_lock, quitting)
    }

    fn apply(&self, path: &Path) -> ExecCommand {
        if let Some((head, tail)) = self.argv.split_first() {
            let prog = clear_stubs(head, path);
            let args = tail.iter().map(|arg| clear_stubs(arg, path)).collect();

            ExecCommand { prog, args }
        } else {
            unreachable!("ExecTemplate is empty")
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

    loop {
        if iter.peek().is_none() {
            break;
        }

        let bytes = iter.by_ref().take_while(|c| c != &&b'{').cloned().collect();
        buffer.push(OsString::from_vec(bytes));
        // TODO: {filename} {basename} {extension} {dirname}
        if iter.peek() == Some(&&b'}') {
            buffer.push(path.as_os_str());
            iter.next();
        }
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

    fn mks(prog: &str) -> OsString {
        OsString::from(prog)
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
        let expected = ExecCommand {
            prog: mks("cp"),
            args: mkv(&["foo", "foo.bak"]),
        };
        assert_eq!(command, expected);
    }
}
