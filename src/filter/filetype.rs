use super::*;

#[derive(Debug)]
pub enum FileType {
    Directory,
    Regular,
    SymLink,
    Executable,
    // TODO: character devices, block devices, sockets, pipes, etc.
}

impl FileType {
    pub fn from_str(symbol: &OsStr) -> Result<FileType, Error> {
        match symbol.to_ascii_lowercase().as_bytes() {
            b"d" | b"directory" => Ok(FileType::Directory),
            b"f" | b"file" => Ok(FileType::Regular),
            b"l" | b"symlink" => Ok(FileType::SymLink),
            b"x" | b"executable" => Ok(FileType::Executable),
            _ => Err(Error::from_str(&format!(
                "found unrecognized file type {:?}",
                symbol
            ))),
        }
    }
}
