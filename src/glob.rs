use super::globset;
use super::regex::bytes::RegexBuilder;

use super::internal::{AppOptions, error};

// http://pubs.opengroup.org/onlinepubs/9699919799/functions/glob.html
pub struct GlobBuilder {}

impl GlobBuilder {
    pub fn new(pattern: &str, config: &AppOptions) -> RegexBuilder {
        match globset::GlobBuilder::new(pattern)
            .unicode(config.unicode)
            .case_insensitive(false)
            .literal_separator(config.match_full_path)
            .build()
        {
            Ok(glob) => RegexBuilder::new(glob.regex()),
            Err(err) => error(&err.to_string()),
        }
    }
}
