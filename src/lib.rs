use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, Read};
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

pub const DEFAULT_PORT: u16 = 8192;

#[derive(Serialize, Deserialize, Debug)]
pub struct RawFileEntry {
    pub is_dry_run: bool,
    pub path_buf: PathBuf,
    pub raw_data: Vec<u8>,
}

pub enum Error {
    IOError(io::Error),
}

impl RawFileEntry {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path_buf = path.as_ref().to_owned();
        let mut file = File::open(path).map_err(Error::IOError)?;
        let mut raw_data = Vec::new();
        file.read_to_end(&mut raw_data).map_err(Error::IOError)?;
        Ok(RawFileEntry {
            path_buf,
            raw_data,
            is_dry_run: false,
        })
    }

    pub fn set_is_dry_run(&mut self, is_dry_run: bool) {
        self.is_dry_run = is_dry_run;
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
