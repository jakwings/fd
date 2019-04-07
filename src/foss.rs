pub use std::ffi::{OsStr, OsString};
pub use std::os::unix::ffi::{OsStrExt, OsStringExt};

pub trait FckOsStrSck {
    fn starts_with(&self, prefix: &OsStr) -> bool;
    fn ends_with(&self, suffix: &OsStr) -> bool;
    fn to_ascii_lowercase(&self) -> OsString;
    fn split_at_comma<'a>(&'a self) -> Vec<&'a OsStr>;
}

impl FckOsStrSck for OsStr {
    fn starts_with(&self, prefix: &OsStr) -> bool {
        self.as_bytes().starts_with(prefix.as_bytes())
    }

    fn ends_with(&self, suffix: &OsStr) -> bool {
        self.as_bytes().ends_with(suffix.as_bytes())
    }

    fn to_ascii_lowercase(&self) -> OsString {
        OsString::from_vec(self.as_bytes().to_ascii_lowercase())
    }

    fn split_at_comma<'a>(&'a self) -> Vec<&'a OsStr> {
        self.as_bytes()
            .split(|&byte| byte == b',')
            .map(|bytes| OsStr::from_bytes(bytes))
            .collect()
    }
}
