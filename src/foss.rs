pub use std::ffi::{OsStr, OsString};
pub use std::os::unix::ffi::{OsStrExt, OsStringExt};

pub trait AsFakingRef<T: ?Sized> {
    fn as_faking_ref(&self) -> &T;
}

impl<T: AsRef<OsStr>> AsFakingRef<[u8]> for T {
    fn as_faking_ref(&self) -> &[u8] {
        self.as_ref().as_bytes()
    }
}

pub trait FckOsStrSck<T: AsFakingRef<[u8]>> {
    fn starts_with(&self, bytes: &T) -> bool;
    fn ends_with(&self, bytes: &T) -> bool;
    fn to_ascii_lowercase(&self) -> OsString;
}

impl<T: AsFakingRef<[u8]>> FckOsStrSck<T> for T {
    fn starts_with(&self, bytes: &T) -> bool {
        self.as_faking_ref().starts_with(bytes.as_faking_ref())
    }

    fn ends_with(&self, bytes: &T) -> bool {
        self.as_faking_ref().ends_with(bytes.as_faking_ref())
    }

    fn to_ascii_lowercase(&self) -> OsString {
        OsString::from_vec(self.as_faking_ref().to_ascii_lowercase())
    }
}
