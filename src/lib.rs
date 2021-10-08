use indicatif::{ProgressBar, ProgressStyle};
use rw::BinaryRead;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::{self, File};
use std::io::{self, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf, StripPrefixError};

#[macro_use]
pub mod macros;

pub mod rw;
pub mod session;
pub mod stream_handler;
pub mod thread_pool;

mod client;
pub use client::run as client;
mod server;
pub use server::run as server;

pub use session::Session;
pub use stream_handler::handle_stream;
pub use thread_pool::ThreadPool;

pub const DEFAULT_IP_ADDRESS: &str = "127.0.0.1";
pub const DEFAULT_PORT: u16 = 8192;

#[derive(Serialize, Deserialize, Debug)]
pub struct TrsferSetting {
    is_dry_run: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FileMetadata {
    pub path_buf: PathBuf,
    pub file_size: u64,
}

#[repr(transparent)]
pub struct FileContent(Vec<u8>);

impl FileContent {
    pub fn load(path: impl AsRef<Path>, progress_bar: &ProgressBar) -> Result<Self, Error> {
        let mut file = File::open(path).map_err(Error::IOError)?;
        let len = file.metadata().map_err(Error::IOError)?.len() as usize;

        let mut buf = Vec::new();
        file.read_with_progress(&mut buf, len, progress_bar)
            .map_err(Error::IOError)?;
        Ok(FileContent(buf))
    }

    pub fn save(&self, save_path: impl AsRef<Path>) -> Result<(), Error> {
        let save_path = save_path.as_ref();
        if let Some(parent_path) = save_path.parent() {
            fs::create_dir_all(parent_path).map_err(Error::IOError)?;
        }
        let mut file = File::create(&save_path).map_err(Error::IOError)?;
        file.write_all(&self.0).map_err(Error::IOError)
    }
}

impl Deref for FileContent {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub enum Error {
    IOError(io::Error),
}

impl FileMetadata {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path_buf = path.as_ref().to_owned();
        let file = File::open(path).map_err(Error::IOError)?;
        let metadata = file.metadata().map_err(Error::IOError)?;
        let file_size = metadata.len();

        Ok(FileMetadata {
            path_buf,
            file_size,
        })
    }

    pub fn strip_prefix(&mut self, base: impl AsRef<Path>) -> Result<(), StripPrefixError> {
        self.path_buf = self.path_buf.strip_prefix(base.as_ref())?.to_owned();
        Ok(())
    }
}

pub struct ChildPaths {
    stack: VecDeque<Result<PathBuf, io::Error>>,
}

impl ChildPaths {
    pub fn from_path(path: impl AsRef<Path>) -> Self {
        let mut stack = VecDeque::new();
        let base = path.as_ref().to_owned();
        stack.push_back(Ok(base));
        ChildPaths { stack }
    }
}

impl Iterator for ChildPaths {
    type Item = Result<PathBuf, io::Error>;

    // TODO: lazily load file entries
    fn next(&mut self) -> Option<Self::Item> {
        match self.stack.pop_back()? {
            Ok(path) => {
                if path.is_dir() {
                    match path.read_dir() {
                        Ok(read_dir) => {
                            self.stack
                                .extend(read_dir.into_iter().map(|e| e.map(|e| e.path())));
                            self.next()
                        }
                        Err(e) => Some(Err(e)),
                    }
                } else {
                    Some(Ok(path))
                }
            }
            e => Some(e),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    pub save_path: String,
}

#[cfg(test)]
mod file_entries {
    use super::ChildPaths;
    use std::path::Path;

    #[test]
    fn current_dir() {
        let path = Path::new("../trsfer-server");
        let file_entries = ChildPaths::from_path(path);
        for path in file_entries {
            match path {
                Ok(path) => {
                    println!("{:?}", path);
                }
                Err(e) => {
                    eprintln!("{:?}", e);
                }
            }
        }
    }
}

pub fn default_progress_style() -> ProgressStyle {
    ProgressStyle::default_bar()
        .template("{msg} [{bar:.cyan/blue}] {bytes}/{total_bytes} ({eta_precise})")
        .progress_chars("##-")
}
