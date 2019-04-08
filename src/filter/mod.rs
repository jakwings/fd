mod filetype;
mod parser;
#[cfg(feature = "reduction")]
mod reduction; // FIXME: experimental!

use super::regex::bytes::Regex;

use super::foss::*;
use super::fshelper::{is_executable, to_absolute_path};
use super::internal::{die, warn, AppOptions, Error};
use super::pattern::PatternBuilder;
use super::walk::DirEntry;

pub use self::filetype::*;

#[derive(Clone, Debug)]
pub enum Action {
    // TODO: FPrint(PathBuf), FPrint0(PathBuf), FdPrint(RawFd), FdPrint0(RawFd), ...
    Print,
    Print0,
}

pub enum Filter {
    // TODO: Size(Range), Depth(usize), Perm(0oXXXX), ...
    Anything, // always true
    Name(Regex),
    Path(Regex),
    Type(FileType),
    Chain(Chain),
    Action(Action), // always true; irreducible unless after short-circuit AND/OR
}

impl std::fmt::Debug for Filter {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match &self {
            Filter::Anything => write!(f, "Anything"),
            Filter::Name(regex) => write!(f, "Name({:?})", regex),
            Filter::Path(regex) => write!(f, "Path({:?})", regex),
            Filter::Type(ftype) => write!(f, "Type({:?})", ftype),
            Filter::Action(action) => write!(f, "Action({:?})", action),
            Filter::Chain(chain) => write!(f, "{:?}", chain),
        }
    }
}

#[derive(Debug, PartialEq)]
enum Joint {
    And, // short-circuit
    Or,  // short-circuit
    Xor,
    Yor, // reset
}

struct Link {
    negated: bool, // negate an individual link
    joint: Joint,
    filter: Filter,
}

impl std::fmt::Debug for Link {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let negation = if self.negated { "!" } else { "" };
        write!(f, "{:?} {}{:?}", self.joint, negation, self.filter)
    }
}

impl Link {
    fn new(joint: Joint, filter: Filter, negated: bool) -> Link {
        Link {
            joint,
            filter,
            negated,
        }
    }
}

pub struct Chain {
    negated: bool, // negate the whole chain
    has_actions: bool,
    links: Vec<Link>, // (((p1 @ p2) @ p3) @ ...)...
}

impl std::fmt::Debug for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let negation = if self.negated { "!" } else { "" };
        let action = if self.has_actions { "@" } else { "" };
        #[cfg(not(test))]
        write!(f, "{}{}Chain(TRUE", negation, action)?;
        #[cfg(test)]
        write!(f, "{}{}(TRUE", negation, action)?;
        for link in &self.links {
            write!(f, " {:?}", link)?;
        }
        write!(f, ")")
    }
}

impl std::default::Default for Chain {
    fn default() -> Chain {
        Chain {
            negated: false,
            has_actions: false,
            links: vec![],
        }
    }
}

impl Chain {
    pub fn new(filter: Filter, negated: bool) -> Chain {
        let chain = Chain::default();

        match (&filter, negated) {
            // empty means (true)
            (Filter::Anything, false) => chain,
            // start with (true AND what) or (true YOR what)
            _ => chain.and(filter, negated),
        }
    }

    pub fn from_args<'a>(
        args: &'a mut impl Iterator<Item = &'a OsStr>,
        config: &'a AppOptions,
    ) -> Result<Chain, Error> {
        let conf = parser::Config {
            unicode: config.unicode,
        };

        parser::Parser::<'a>::new(args, conf)
            .parse()
            .map(|chain| Chain::reduce(chain))
    }

    pub fn reduce(chain: Chain) -> Chain {
        #[cfg(feature = "reduction")]
        return reduction::reduce(chain);
        #[cfg(not(feature = "reduction"))]
        return chain;
    }

    // NOTE: x.not().or(y) == x.or(y).not()
    pub fn not(mut self) -> Chain {
        self.negated ^= true;
        self
    }

    pub fn and(self, filter: Filter, negated: bool) -> Chain {
        self.push(Link::new(Joint::And, filter, negated))
    }

    pub fn or(self, filter: Filter, negated: bool) -> Chain {
        self.push(Link::new(Joint::Or, filter, negated))
    }

    pub fn xor(self, filter: Filter, negated: bool) -> Chain {
        self.push(Link::new(Joint::Xor, filter, negated))
    }

    pub fn yor(self, filter: Filter, negated: bool) -> Chain {
        self.push(Link::new(Joint::Yor, filter, negated))
    }

    // and no pop()
    fn push(mut self, link: Link) -> Chain {
        match link.filter {
            Filter::Action(_) => self.has_actions = true,
            Filter::Chain(ref chain) if chain.has_actions => self.has_actions = true,
            _ => (),
        }
        self.links.push(link);
        self
    }

    fn bool(joint: &Joint, lhs: bool, rhs: bool) -> bool {
        match joint {
            Joint::And => lhs & rhs,
            Joint::Or => lhs | rhs,
            Joint::Xor => lhs ^ rhs,
            Joint::Yor => rhs,
        }
    }

    pub fn apply(&self, entry: &DirEntry, config: &AppOptions) -> Vec<Action> {
        let mut actions = Vec::new();

        if self.test(entry, config, &mut actions) {
            if actions.is_empty() && !self.has_actions {
                if config.null_terminator {
                    actions.push(Action::Print0);
                } else {
                    actions.push(Action::Print);
                }
            }
        }

        actions
    }

    fn test(&self, entry: &DirEntry, config: &AppOptions, actions: &mut Vec<Action>) -> bool {
        let mut result = true;

        for link in &self.links {
            result = match (result, &link.joint) {
                (false, Joint::And) | (true, Joint::Or) => result,
                _ => match &link.filter {
                    Filter::Name(ref pattern) => Chain::bool(
                        &link.joint,
                        result,
                        self.test_pattern(pattern, entry, false, false) ^ link.negated,
                    ),
                    Filter::Path(ref pattern) => Chain::bool(
                        &link.joint,
                        result,
                        self.test_pattern(pattern, entry, true, config.match_full_path)
                            ^ link.negated,
                    ),
                    Filter::Type(ref ftype) => Chain::bool(
                        &link.joint,
                        result,
                        self.test_filetype(ftype, entry) ^ link.negated,
                    ),
                    Filter::Chain(ref chain) => Chain::bool(
                        &link.joint,
                        result,
                        chain.test(entry, config, actions) ^ link.negated,
                    ),
                    Filter::Action(ref action) => {
                        actions.push(action.to_owned());
                        Chain::bool(&link.joint, result, true ^ link.negated)
                    }
                    Filter::Anything => Chain::bool(&link.joint, result, true ^ link.negated),
                },
            }
        }

        result ^ self.negated
    }

    fn test_pattern(
        &self,
        pattern: &Regex,
        entry: &DirEntry,
        match_path: bool,
        match_full_path: bool,
    ) -> bool {
        let entry_path = entry.path;

        if match_full_path {
            if let Ok(path_buf) = to_absolute_path(&entry_path) {
                return pattern.is_match(path_buf.as_os_str().as_bytes());
            } else {
                die(&format!(
                    "could not get full path of {:?}",
                    entry_path.as_os_str()
                ));
            }
        } else if match_path {
            return pattern.is_match(entry_path.as_os_str().as_bytes());
        } else {
            if let Some(os_str) = entry_path.file_name() {
                return pattern.is_match(os_str.as_bytes());
            }
        }

        false
    }

    fn test_filetype(&self, ftype: &FileType, entry: &DirEntry) -> bool {
        let entry_path = entry.path;

        if let Some(ref file_type) = entry.file_type {
            match ftype {
                // only zero or one of is_dir/is_file/is_symlink can be true
                FileType::Directory => return file_type.is_dir(),
                FileType::Regular => return file_type.is_file(),
                FileType::SymLink => return file_type.is_symlink(),
                // only accept likely-execve(2)-able files
                FileType::Executable => {
                    // entry_path.metadata() always follows symlinks
                    return if let Ok(meta) = entry_path.metadata() {
                        // also exclude symlinks to directories
                        !meta.is_dir()
                            // exclude character devices, block devices, sockets, pipes, etc.
                            && (file_type.is_file() || file_type.is_symlink())
                            // with the execute permission file mode bits set
                            && is_executable(&meta)
                    } else {
                        if !file_type.is_symlink() {
                            // permission denied?
                            warn(&format!(
                                "could not get metadata of {:?}",
                                entry_path.as_os_str()
                            ));
                        } // else: symlinks to non-existent files

                        false
                    };
                }
            }
        } else {
            warn(&format!(
                "could not get file type of {:?}",
                entry_path.as_os_str()
            ));
            return false;
        }
    }

    #[cfg(test)]
    fn test_logic(&self) -> bool {
        self.links
            .iter()
            .by_ref()
            .fold(true, |result, link| match (result, &link.joint) {
                (false, Joint::And) | (true, Joint::Or) => result,
                _ => match &link.filter {
                    Filter::Anything | Filter::Action(_) => {
                        Chain::bool(&link.joint, result, true ^ link.negated)
                    }
                    Filter::Chain(ref chain) => {
                        Chain::bool(&link.joint, result, chain.test_logic() ^ link.negated)
                    }
                    _ => unreachable!(),
                },
            })
            ^ self.negated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! chain (($x:expr) => (Chain::new(Filter::Anything, !$x)));
    macro_rules! action (($x:expr) => (Chain::new(Filter::Action(Action::Print), !$x)));

    macro_rules! and (($x:expr, $y:expr) => (chain!($x).and(Filter::Anything, !$y)));
    macro_rules! or (($x:expr, $y:expr) => (chain!($x).or(Filter::Anything, !$y)));
    macro_rules! xor (($x:expr, $y:expr) => (chain!($x).xor(Filter::Anything, !$y)));
    macro_rules! yor (($x:expr, $y:expr) => (chain!($x).yor(Filter::Anything, !$y)));

    macro_rules! nand (($x:expr, $y:expr) => (and!($x, $y).not()));
    macro_rules! nor (($x:expr, $y:expr) => (or!($x, $y).not()));
    macro_rules! nxor (($x:expr, $y:expr) => (xor!($x, $y).not()));
    macro_rules! nyor (($x:expr, $y:expr) => (yor!($x, $y).not()));

    macro_rules! cand (($x:expr, $y:expr) => ($x.and(Filter::Chain($y), false)));
    macro_rules! cor (($x:expr, $y:expr) => ($x.or(Filter::Chain($y), false)));
    macro_rules! cxor (($x:expr, $y:expr) => ($x.xor(Filter::Chain($y), false)));
    macro_rules! cyor (($x:expr, $y:expr) => ($x.yor(Filter::Chain($y), false)));

    macro_rules! mand (($($x:expr),+) => {
        Chain::default()
            $(.and(Filter::Action(Action::Print), !$x))+
    });

    macro_rules! mor (($($x:expr),+) => {
        Chain::new(Filter::Anything, true)
            $(.or(Filter::Action(Action::Print), !$x))+
    });

    macro_rules! mxor (($($x:expr),+) => {
        Chain::default()
            $(.xor(Filter::Action(Action::Print), !$x))+
    });

    macro_rules! myor (($($x:expr),+) => {
        Chain::default()
            $(.yor(Filter::Action(Action::Print), !$x))+
    });

    macro_rules! oops (($op1:ident $x:expr, $op2:ident $($y:expr)+) => {
        Chain::default()
            .$op1(Filter::Action(Action::Print), !$x)
            $(.$op2(Filter::Action(Action::Print), !$y))+
    });

    macro_rules! calc {
        ($expected:expr, $x:expr) => {
            let s = format!("N {:>5} = {:?}", $expected, $x);
            assert_eq!($expected, $x.test_logic(), "\n{}\n", s);
            let x = Chain::reduce($x);
            let s = format!("{}\nR {:>5} = {:?}", s, $expected, x);
            assert_eq!($expected, x.test_logic(), "\n{}\n", s);
        };
    }

    #[test]
    #[rustfmt::skip::macros(calc)]
    fn filter_logic() {
        calc!(true, chain!(true));
        calc!(false, chain!(false));
        calc!(false, chain!(true).not());
        calc!(true, chain!(false).not());

        calc!(true, and!(true, true));
        calc!(false, and!(true, false));
        calc!(false, and!(false, true));
        calc!(false, and!(false, false));

        calc!(true, or!(true, true));
        calc!(true, or!(true, false));
        calc!(true, or!(false, true));
        calc!(false, or!(false, false));

        calc!(false, xor!(true, true));
        calc!(true, xor!(true, false));
        calc!(true, xor!(false, true));
        calc!(false, xor!(false, false));

        calc!(true, yor!(true, true));
        calc!(false, yor!(true, false));
        calc!(true, yor!(false, true));
        calc!(false, yor!(false, false));

        calc!(false, nand!(true, true));
        calc!(true, nand!(true, false));
        calc!(true, nand!(false, true));
        calc!(true, nand!(false, false));

        calc!(false, nor!(true, true));
        calc!(false, nor!(true, false));
        calc!(false, nor!(false, true));
        calc!(true, nor!(false, false));

        calc!(true, nxor!(true, true));
        calc!(false, nxor!(true, false));
        calc!(false, nxor!(false, true));
        calc!(true, nxor!(false, false));

        calc!(false, nyor!(true, true));
        calc!(true, nyor!(true, false));
        calc!(false, nyor!(false, true));
        calc!(true, nyor!(false, false));

        calc!(true, cxor!(and!(false, true), chain!(true)));
        calc!(false, cxor!(or!(true, false), chain!(true)));
        calc!(false, cxor!(xor!(true, false), chain!(true)));
        calc!(false, cxor!(yor!(false, true), chain!(true)));

        calc!(true, cor!(and!(false, true), chain!(true)));
        calc!(true, cyor!(and!(false, true), chain!(true)));
        calc!(false, cand!(or!(true, false), chain!(false)));
        calc!(false, cyor!(or!(true, false), chain!(false)));

        calc!(false, cand!(chain!(true), chain!(true).not()));
        calc!(true, cand!(chain!(true), chain!(false).not()));
        calc!(false, cand!(chain!(false), chain!(true).not()));
        calc!(false, cand!(chain!(false), chain!(false).not()));

        calc!(true, cor!(chain!(true), chain!(true).not()));
        calc!(true, cor!(chain!(true), chain!(false).not()));
        calc!(false, cor!(chain!(false), chain!(true).not()));
        calc!(true, cor!(chain!(false), chain!(false).not()));

        calc!(true, cxor!(chain!(true), chain!(true).not()));
        calc!(false, cxor!(chain!(true), chain!(false).not()));
        calc!(false, cxor!(chain!(false), chain!(true).not()));
        calc!(true, cxor!(chain!(false), chain!(false).not()));

        calc!(false, cyor!(chain!(true), chain!(true).not()));
        calc!(true, cyor!(chain!(true), chain!(false).not()));
        calc!(false, cyor!(chain!(false), chain!(true).not()));
        calc!(true, cyor!(chain!(false), chain!(false).not()));

        // cand!(x,y).not()
        calc!(false, cand!(chain!(true).not(), chain!(true)));
        calc!(true, cand!(chain!(true).not(), chain!(false)));
        calc!(true, cand!(chain!(false).not(), chain!(true)));
        calc!(true, cand!(chain!(false).not(), chain!(false)));

        // cor!(x,y).not()
        calc!(false, cor!(chain!(true).not(), chain!(true)));
        calc!(false, cor!(chain!(true).not(), chain!(false)));
        calc!(false, cor!(chain!(false).not(), chain!(true)));
        calc!(true, cor!(chain!(false).not(), chain!(false)));

        // cxor!(x,y).not()
        calc!(true, cxor!(chain!(true).not(), chain!(true)));
        calc!(false, cxor!(chain!(true).not(), chain!(false)));
        calc!(false, cxor!(chain!(false).not(), chain!(true)));
        calc!(true, cxor!(chain!(false).not(), chain!(false)));

        // cyor!(x,y).not()
        calc!(false, cyor!(chain!(true).not(), chain!(true)));
        calc!(true, cyor!(chain!(true).not(), chain!(false)));
        calc!(false, cyor!(chain!(false).not(), chain!(true)));
        calc!(true, cyor!(chain!(false).not(), chain!(false)));

        calc!(false, cand!(chain!(true), cand!(chain!(true), action!(true))).not());
        calc!(false, cand!(chain!(true), cor!(chain!(true), action!(true))).not());
        calc!(true, cand!(chain!(true), cxor!(chain!(true), action!(true))).not());
        calc!(false, cand!(chain!(true), cyor!(chain!(true), action!(true))).not());

        calc!(false, cand!(action!(true), mand!(true, true, false)));
        calc!(false, cor!(action!(false), mand!(true, true, false)));
        calc!(true, cxor!(action!(true), mand!(false, false, true)));
        calc!(true, cyor!(action!(false), mand!(true, true, true)));
        calc!(false, cand!(action!(true), mand!(true, true, true).not()));
        calc!(false, cor!(action!(false), mand!(true, true, true).not()));
        calc!(false, cxor!(action!(true), mand!(false, false, false).not()));
        calc!(false, cyor!(action!(false), mand!(true, true, true).not()));
        calc!(false, cand!(action!(true), mand!(false)));
        calc!(true, cand!(action!(true), mand!(false).not()));
        calc!(false, cor!(action!(false), mand!(false)));
        calc!(true, cor!(action!(false), mand!(false).not()));
        calc!(false, cxor!(action!(false), mand!(false)));
        calc!(true, cxor!(action!(false), mand!(false).not()));
        calc!(false, cyor!(action!(true), mand!(false)));
        calc!(true, cyor!(action!(true), mand!(false).not()));

        calc!(false, cand!(action!(true), mor!(false, false, false)));
        calc!(false, cor!(action!(false), mor!(false, false, false)));
        calc!(false, cxor!(action!(true), mor!(true, false, false)));
        calc!(true, cyor!(action!(false), mor!(true, false, false)));
        calc!(true, cand!(action!(true), mor!(false, false, false).not()));
        calc!(true, cor!(action!(false), mor!(false, false, false).not()));
        calc!(true, cxor!(action!(true), mor!(true, false, false).not()));
        calc!(false, cyor!(action!(false), mor!(false, true, false).not()));
        calc!(false, cand!(action!(true), mor!(false)));
        calc!(true, cand!(action!(true), mor!(false).not()));
        calc!(true, cor!(action!(false), mor!(true)));
        calc!(false, cor!(action!(false), mor!(true).not()));
        calc!(false, cxor!(action!(false), mor!(false)));
        calc!(true, cxor!(action!(false), mor!(false).not()));
        calc!(false, cyor!(action!(true), mor!(false)));
        calc!(true, cyor!(action!(true), mor!(false).not()));

        calc!(false, cand!(action!(true), mxor!(false, true, false)));
        calc!(true, cor!(action!(true), mxor!(true, false, true)));
        calc!(true, cxor!(action!(false), mxor!(true, false, true)));
        calc!(true, cyor!(action!(true), mxor!(true, false, true)));
        calc!(true, cand!(action!(true), mxor!(false, true, false).not()));
        calc!(true, cor!(action!(true), mxor!(true, false, true).not()));
        calc!(false, cxor!(action!(false), mxor!(true, false, true).not()));
        calc!(false, cyor!(action!(true), mxor!(true, false, true).not()));
        calc!(false, cand!(action!(true), mxor!(true)));
        calc!(true, cand!(action!(true), mxor!(true).not()));
        calc!(false, cor!(action!(false), mxor!(true)));
        calc!(true, cor!(action!(false), mxor!(true).not()));
        calc!(false, cxor!(action!(false), mxor!(true)));
        calc!(false, cxor!(action!(false), mxor!(false).not()));
        calc!(false, cyor!(action!(true), mxor!(true)));
        calc!(true, cyor!(action!(true), mxor!(true).not()));

        calc!(false, cand!(action!(false), myor!(false, false, true)));
        calc!(true, cor!(action!(true), myor!(true, true, false)));
        calc!(false, cxor!(action!(true), myor!(false, false, true)));
        calc!(false, cyor!(action!(true), myor!(false, true, false)));
        calc!(true, cand!(action!(true), myor!(false, true, false).not()));
        calc!(false, cor!(action!(false), myor!(true, false, true).not()));
        calc!(false, cxor!(action!(true), myor!(false, true, false).not()));
        calc!(true, cyor!(action!(false), myor!(false, true, false).not()));
        calc!(false, cand!(action!(false), myor!(true)));
        calc!(false, cand!(action!(false), myor!(false).not()));
        calc!(true, cor!(action!(true), myor!(false)));
        calc!(true, cor!(action!(true), myor!(false).not()));
        calc!(true, cxor!(action!(true), myor!(false)));
        calc!(false, cxor!(action!(true), myor!(false).not()));
        calc!(false, cyor!(action!(true), myor!(false)));
        calc!(true, cyor!(action!(true), myor!(false).not()));

        calc!(false, cxor!(action!(true), oops!(and false, xor false false true)));
        calc!(false, cxor!(action!(true), oops!(xor true, xor false false true)));
        calc!(false, cxor!(action!(true), oops!(yor false, xor false false true)));
        calc!(true, cxor!(action!(true), oops!(and false, xor false false true).not()));
        calc!(true, cxor!(action!(true), oops!(xor true, xor false false true).not()));
        calc!(true, cxor!(action!(true), oops!(yor false, xor false false true).not()));

        calc!(false, cyor!(action!(false), oops!(and true, yor false true false)));
        calc!(false, cyor!(action!(false), oops!(or true, yor false true false)));
        calc!(false, cyor!(action!(false), oops!(xor true, yor false true false)));
        calc!(false, cyor!(action!(false), oops!(yor true, yor false true false)));
        calc!(true, cyor!(action!(false), oops!(and true, yor false true false).not()));
        calc!(true, cyor!(action!(false), oops!(or true, yor false true false).not()));
        calc!(true, cyor!(action!(false), oops!(xor true, yor false true false).not()));
        calc!(true, cyor!(action!(false), oops!(yor true, yor false true false).not()));
    }
}
