use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;

use super::regex::bytes::{Regex, RegexBuilder};

use super::glob::GlobBuilder;
use super::internal::Error;

pub struct PatternBuilder {
    pattern: OsString,
    use_regex: bool,
    unicode: bool,
    case_insensitive: bool,
    match_full_path: bool,
}

impl PatternBuilder {
    pub fn new(pattern: &OsStr) -> PatternBuilder {
        PatternBuilder {
            pattern: pattern.to_os_string(),
            use_regex: false,
            unicode: false,
            case_insensitive: false,
            match_full_path: false,
        }
    }

    pub fn build(&self) -> Result<Regex, Error> {
        if self.use_regex {
            // XXX: strange conformance to UTF-8
            let pattern = if self.unicode {
                self.pattern
                    .clone()
                    .into_string()
                    .or(Err(Error::from_str("need a UTF-8 encoded pattern")))?
            } else {
                PatternBuilder::escape_pattern(&self.pattern)
                    .ok_or(Error::from_str("invalid UTF-8 byte sequences found"))?
            };

            //      (?u)π or (?u:π) doesn't match π without --unicode?
            //      (?-u:π) is not allowed with --unicode?
            RegexBuilder::new(&pattern)
                .unicode(self.unicode)
                .case_insensitive(self.case_insensitive)
                .dot_matches_new_line(true)
                .build()
                .map_err(|err| Error::from_str(&err.to_string()))
        } else {
            GlobBuilder::new(&self.pattern)
                .unicode(self.unicode)
                .case_insensitive(self.case_insensitive)
                .match_full_path(self.match_full_path)
                .build()
        }
    }

    pub fn use_regex(mut self, on: bool) -> PatternBuilder {
        self.use_regex = on;
        self
    }

    pub fn unicode(mut self, on: bool) -> PatternBuilder {
        self.unicode = on;
        self
    }

    pub fn case_insensitive(mut self, on: bool) -> PatternBuilder {
        self.case_insensitive = on;
        self
    }

    pub fn match_full_path(mut self, on: bool) -> PatternBuilder {
        self.match_full_path = on;
        self
    }

    // TODO: patch "regex" or "regex-syntax", or use another engine
    // The regex crate can't help much: https://github.com/rust-lang/regex/issues/426
    // The man asked my use case again and again, but I found that guy case-insensitive.
    fn escape_pattern(pattern: &OsStr) -> Option<String> {
        let mut bytes = Vec::new();

        for c in pattern.as_bytes() {
            let c = *c;

            if c <= 0x1F || c >= 0x7F {
                let buff = format!("\\x{:02X}", c);

                bytes.append(&mut buff.into_bytes());
            } else {
                bytes.push(c);
            }
        }

        String::from_utf8(bytes).ok()
    }
}
