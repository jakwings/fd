use super::globset;
use super::regex::bytes::RegexBuilder;

use super::internal::fatal;

// http://pubs.opengroup.org/onlinepubs/9699919799/functions/glob.html
// https://docs.rs/globset/latest/globset/#syntax
pub struct GlobBuilder {}

impl GlobBuilder {
    pub fn new(pattern: &str, unicode: bool, full_path: bool) -> RegexBuilder {
        match globset::GlobBuilder::new(pattern)
            .unicode(unicode)
            .backslash_escape(true)
            .case_insensitive(false)
            .literal_separator(full_path)
            .build()
        {
            Ok(glob) => RegexBuilder::new(glob.regex()),
            Err(err) => fatal(&format!("failed to parse glob pattern: {}", err)),
        }
    }
}
