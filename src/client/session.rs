use std::io;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::sync::Arc;

use super::thread_pool::ThreadPool;

use crate::rw::{BinaryRead, BinaryWrite};
use crate::{exit, ChildPaths, FileContent, FileMetadata, Response};
use crate::{
    set_error_style, set_load_style_begin, set_load_style_end, set_send_style_begin,
    set_send_style_end, set_skipped_style, Error, Result, TrsferSetting,
};

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
            multi_progress,
            thread_pool,
        })
    }

    pub fn run_by_path(&self, base_path: impl AsRef<Path>, is_dry_run: bool) -> Result<()> {
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

            self.thread_pool.execute(move |stream| {
                let task = || -> Result<()> {
                    let setting = TrsferSetting { is_dry_run };

                    let mut file_metadata = match FileMetadata::load(&path) {
                        Ok(file_metadata) => file_metadata,
                        _ => {
                            panic!("failed to open `{}`", path.to_string_lossy().to_owned());
                        }
                    };
                    file_metadata
                        .strip_prefix(&*base_path)
                        .map_err(Error::StripPrefixError)?;

                    // set progress bar message
                    set_load_style_begin(&progress_bar);

                    progress_bar.set_message(file_metadata.path_buf.to_string_lossy().into_owned());

                    // create read/write stream
                    let stream_cloned = stream.try_clone().map_err(Error::IOError)?;
                    let writer = stream;
                    let mut reader = stream_cloned;

                    // send request
                    writer.write_serialize(&setting)?;
                    writer.write_serialize(&file_metadata)?;

                    let mut buf = Vec::new();
                    let file_exists = reader.read_deserialize::<bool>(&mut buf)?;
                    if file_exists {
                        set_skipped_style(&progress_bar);
                        progress_bar.finish_with_message(
                            file_metadata.path_buf.to_string_lossy().to_string(),
                        );
                        return Ok(());
                    }

                    // create raw data
                    let content = FileContent::load(&path, &progress_bar)?;

                    set_load_style_end(&progress_bar);

                    set_send_style_begin(&progress_bar);

                    writer
                        .write_binary_with_progress(&content, &progress_bar)
                        .map_err(Error::IOError)?;

                    set_send_style_end(&progress_bar);

                    progress_bar
                        .finish_with_message(file_metadata.path_buf.to_string_lossy().to_string());

                    // receive response
                    let mut buf = Vec::new();
                    reader.read_binary(&mut buf).map_err(Error::IOError)?;
                    let _response =
                        bincode::deserialize::<Response>(&buf).map_err(Error::BincodeError)?;

                    Ok(())
                };

                if let Err(e) = task() {
                    set_error_style(&progress_bar, e);
                }
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
