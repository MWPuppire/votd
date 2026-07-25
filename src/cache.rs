use filetime::FileTime;
use std::fmt;
use std::fs;
use std::io::prelude::*;

use crate::api::Verse;

pub const CACHE_EXPIRE_TIME: i64 = 21600; // 6 hours, in seconds

#[derive(Debug)]
pub enum Error {
    NoPath,
    FsError(std::io::Error),
    DecodeError(rmp_serde::decode::Error),
    EncodeError(rmp_serde::encode::Error),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::FsError(err)
    }
}
impl From<rmp_serde::decode::Error> for Error {
    fn from(err: rmp_serde::decode::Error) -> Self {
        Error::DecodeError(err)
    }
}
impl From<rmp_serde::encode::Error> for Error {
    fn from(err: rmp_serde::encode::Error) -> Self {
        Error::EncodeError(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::NoPath => f.write_str("Couldn't determine file path for cache"),
            Self::FsError(e) => write!(f, "File system error for cache: {}", e),
            Self::DecodeError(e) => write!(f, "Error decoding cache file: {}", e),
            Self::EncodeError(e) => write!(f, "Error encoding to cache file: {}", e),
        }
    }
}

impl std::error::Error for Error { }

#[derive(Debug)]
pub struct Cache {
    file: fs::File,
    live: bool,
}

impl Cache {
    pub fn new() -> Result<Self, Error> {
        let path = directories::BaseDirs::new()
            .map(|dirs| dirs.cache_dir().join("votd-cli-cache.txt"))
            .ok_or(Error::NoPath)?;

        let mut opts = fs::OpenOptions::new();
        // first try to open file to read-write
        opts
            .read(true)
            .write(true)
            .create(false)
            .truncate(false)
            .append(false);

        let mut live = true;

        let mut file = match opts.open(&path) {
            Ok(file) => file,
            Err(err) => {
                // if file wasn't found, then try creating it,
                // and mark cache as dead to not read an empty file
                if err.kind() == std::io::ErrorKind::NotFound {
                    live = false;
                    opts.create(true);
                    opts.open(&path)?
                } else {
                    return Err(err.into());
                }
            }
        };

        file.rewind()?;

        let metadata = file.metadata()?;
        let stamp = FileTime::from_last_modification_time(&metadata).seconds();
        let now = FileTime::now().seconds();
        live = live && now - stamp <= CACHE_EXPIRE_TIME;

        Ok(Cache { file, live })
    }

    pub fn is_live(&self) -> bool {
        self.live
    }

    pub fn read(&mut self) -> Result<Vec<Verse>, Error> {
        let mut buf = Vec::new();
        self.file.read_to_end(&mut buf)?;
        Ok(rmp_serde::from_slice(&buf)?)
    }

    pub fn write(&mut self, verses: &[Verse]) -> Result<(), Error> {
        rmp_serde::encode::write(&mut self.file, verses)?;
        Ok(())
    }
}
