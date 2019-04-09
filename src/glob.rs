use std::ffi::{OsStr, OsString};

use super::globset;
use super::internal::Error;

// http://pubs.opengroup.org/onlinepubs/9699919799/functions/glob.html
// https://docs.rs/ff-find/latest/globset/#syntax

pub struct Glob {
    pattern: String,
    matcher: globset::GlobMatcher,
}

impl std::fmt::Debug for Glob {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "{:?}", self.pattern)
    }
}

impl Glob {
    pub fn is_match(&self, path: impl AsRef<OsStr>) -> bool {
        self.matcher.is_match(path.as_ref())
    }
}

pub struct GlobBuilder {
    pattern: OsString,
    unicode: bool,
    case_insensitive: bool,
    match_full_path: bool,
}

impl GlobBuilder {
    pub fn new(pattern: &OsStr) -> GlobBuilder {
        GlobBuilder {
            pattern: pattern.to_os_string(),
            unicode: false,
            case_insensitive: false,
            match_full_path: false,
        }
    }

    pub fn build(&self) -> Result<Glob, Error> {
        // XXX: strange conformance to UTF-8
        let pattern =
            OsStr::to_str(&self.pattern).ok_or(Error::from_str("need a UTF-8 encoded pattern"))?;

        globset::GlobBuilder::new(pattern)
            .unicode(self.unicode)
            .backslash_escape(true)
            .case_insensitive(self.case_insensitive)
            .literal_separator(self.match_full_path)
            .build()
            .map(|glob| Glob {
                pattern: glob.glob().to_string(),
                matcher: glob.compile_matcher(),
            })
            .map_err(|err| Error::from_str(&err.to_string()))
    }

    pub fn unicode(mut self, on: bool) -> GlobBuilder {
        self.unicode = on;
        self
    }

    pub fn case_insensitive(mut self, on: bool) -> GlobBuilder {
        self.case_insensitive = on;
        self
    }

    pub fn match_full_path(mut self, on: bool) -> GlobBuilder {
        self.match_full_path = on;
        self
    }
}
