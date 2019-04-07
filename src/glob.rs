use std::ffi::{OsStr, OsString};

use super::globset;
use super::regex::bytes::{Regex, RegexBuilder};

use super::internal::Error;

// http://pubs.opengroup.org/onlinepubs/9699919799/functions/glob.html
// https://docs.rs/globset/latest/globset/#syntax
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

    pub fn build(&self) -> Result<Regex, Error> {
        // XXX: strange conformance to UTF-8
        let pattern =
            OsStr::to_str(&self.pattern).ok_or(Error::from_str("need a UTF-8 encoded pattern"))?;

        globset::GlobBuilder::new(pattern)
            .unicode(self.unicode)
            .backslash_escape(true)
            .case_insensitive(false)
            .literal_separator(self.match_full_path)
            .build()
            .map_err(|err| Error::from_str(&err.to_string()))
            .and_then(|glob| {
                let mut builder = RegexBuilder::new(glob.regex());

                builder
                    .unicode(self.unicode)
                    .case_insensitive(self.case_insensitive)
                    .dot_matches_new_line(true)
                    .build()
                    .map_err(|err| Error::from_str(&err.to_string()))
            })
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
