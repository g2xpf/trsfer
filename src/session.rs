use std::io;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;

use std::path::Path;

use super::rw::{BinaryRead, BinaryReader, BinaryWrite, BinaryWriter};
use super::{exit, ChildPaths, RawFileEntry, Response};

use crate::ThreadPool;

pub struct Session {
    thread_pool: ThreadPool<TcpStream>,
}

impl Session {
    pub fn connect(num_threads: usize, addr: impl ToSocketAddrs) -> io::Result<Self> {
        let thread_pool = ThreadPool::new(num_threads, || match TcpStream::connect(&addr) {
            Ok(stream) => stream,
            _ => {
                exit!(5, addr.to_socket_addrs().unwrap().next().unwrap());
            }
        });
        Ok(Session { thread_pool })
    }

    pub fn run_by_path(&self, base_path: impl AsRef<Path>, is_dry_run: bool) -> io::Result<()> {
        let child_paths = ChildPaths::from_path(&base_path);
        let base_path = Arc::new(
            base_path
                .as_ref()
                .parent()
                .expect("cannot copy /")
                .to_owned(),
        );

        for path in child_paths {
            let path = path?;
            let base_path = Arc::clone(&base_path);
            self.thread_pool.execute(move |stream| {
                let mut entry = match RawFileEntry::load(&path) {
                    Ok(entry) => entry,
                    _ => {
                        panic!("failed to open `{}`", path.to_str().unwrap());
                    }
                };
                entry.strip_prefix(&*base_path).unwrap();
                entry.set_is_dry_run(is_dry_run);

                let data = match bincode::serialize(&entry) {
                    Ok(data) => data,
                    Err(e) => panic!("{}", e),
                };

                let stream_cloned = stream.try_clone().unwrap();
                let mut writer = BinaryWriter::new(stream);
                let mut reader = BinaryReader::new(stream_cloned);

                // send request
                writer.write_binary(&data).unwrap();

                // receive response
                let mut buf = Vec::new();
                reader.read_binary(&mut buf).unwrap();
                let response = bincode::deserialize::<Response>(&buf).unwrap();

                let dry_run_header = if is_dry_run { "[dry-run] " } else { "" };
                log::info!(
                    "{}`{}` -> `{}`",
                    dry_run_header,
                    path.to_string_lossy(),
                    response.save_path
                );
            });
        }
        Ok(())
    }
}
