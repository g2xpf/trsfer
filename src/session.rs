use std::io;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::sync::Arc;

use super::rw::{BinaryRead, BinaryWrite};
use super::{exit, ChildPaths, FileContent, FileMetadata, Response};

use crate::{default_progress_style, ThreadPool, TrsferSetting};

use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget};

pub struct Session {
    multi_progress: MultiProgress,
    thread_pool: ThreadPool<TcpStream>,
}

impl Session {
    pub fn connect(num_threads: usize, addr: impl ToSocketAddrs) -> io::Result<Self> {
        let multi_progress = MultiProgress::new();
        multi_progress.set_draw_target(ProgressDrawTarget::stdout());

        let thread_pool = ThreadPool::new(num_threads, || {
            let stream = match TcpStream::connect(&addr) {
                Ok(stream) => stream,
                _ => {
                    exit!(5, addr.to_socket_addrs().unwrap().next().unwrap());
                }
            };
            stream
        });
        Ok(Session {
            thread_pool,
            multi_progress,
        })
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
            let progress_bar = self.multi_progress.add(ProgressBar::new(!0));

            progress_bar.set_style(default_progress_style());

            self.thread_pool.execute(move |stream| {
                let setting = TrsferSetting { is_dry_run };

                let mut file_metadata = match FileMetadata::load(&path) {
                    Ok(file_metadata) => file_metadata,
                    _ => {
                        panic!("failed to open `{}`", path.to_str().unwrap());
                    }
                };
                file_metadata.strip_prefix(&*base_path).unwrap();

                // set progress bar message
                progress_bar.set_message(file_metadata.path_buf.to_string_lossy().into_owned());

                // create raw data
                let raw_setting = bincode::serialize(&setting).unwrap();
                let raw_file_metadata = bincode::serialize(&file_metadata).unwrap();
                let content = FileContent::load(&path, &progress_bar).unwrap();

                // create read/write stream
                let stream_cloned = stream.try_clone().unwrap();
                let writer = stream;
                let mut reader = stream_cloned;

                // send request
                writer.write_binary(&raw_setting).unwrap();
                writer.write_binary(&raw_file_metadata).unwrap();
                writer
                    .write_binary_with_progress(&content, &progress_bar)
                    .unwrap();

                progress_bar
                    .finish_with_message(file_metadata.path_buf.to_string_lossy().to_string());

                // receive response
                let mut buf = Vec::new();
                reader.read_binary(&mut buf).unwrap();
                let _response = bincode::deserialize::<Response>(&buf).unwrap();
            });
        }
        Ok(())
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.multi_progress.join().unwrap();
    }
}
