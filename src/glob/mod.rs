use super::globset;
use super::regex::bytes::RegexBuilder;

use super::internal::error;

// http://pubs.opengroup.org/onlinepubs/9699919799/functions/glob.html
//
// TODO: Make a new fork of globset? With a simpler rule set.
// Custom rules:
// 1. "\" removes special meaning of any single following character, then be discarded.
// 2. No character class expression?
// 3. Do not skip dot-files.
// 4. Ignore system locales.
//
pub struct GlobBuilder {}

impl GlobBuilder {
    pub fn new(pattern: &str, search_full_path: bool) -> RegexBuilder {
        match globset::GlobBuilder::new(pattern)
            .literal_separator(search_full_path)
            .build()
        {
            Ok(glob) => {
                // TODO: How to enable Unicode support?
                //       Should rewrite the crate but this is not an easy job due to UTF-8.
                //use std::ops::Index;
                //let stub = "(?-u)";
                //let mut pattern = glob.regex();
                //if pattern.starts_with(stub) {
                //    pattern = pattern.index(stub.len()..);
                //}
                let pattern = glob.regex();

                RegexBuilder::new(pattern)
            }
            Err(err) => error(&err.to_string()),
        }
    }
}
