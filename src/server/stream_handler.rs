use std::net::TcpStream;

use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use crate::rw::{BinaryRead, BinaryWrite};
use crate::{Error, FileContent, FileMetadata, Response, Result, TrsferSetting};

pub fn handle_stream(stream: TcpStream, output_path: Arc<PathBuf>) -> Result<()> {
    let stream_cloned = stream.try_clone().map_err(Error::IOError)?;
    let mut reader = stream;
    let mut writer = stream_cloned;

    loop {
        // receive request
        let mut buf = Vec::new();
        let setting = reader.read_deserialize::<TrsferSetting>(&mut buf)?;

        let mut buf = Vec::new();
        let file_metadata = reader.read_deserialize::<FileMetadata>(&mut buf)?;

        let save_path = output_path.join(&file_metadata.path_buf);

        let exists_file = save_path.exists();
        writer.write_serialize(&exists_file)?;
        if exists_file {
            continue;
        }

        // set progress bar message
        let mut buf = Vec::new();
        reader.read_binary(&mut buf).map_err(Error::IOError)?;
        let file_content = FileContent(buf);

        if !setting.is_dry_run {
            let save_path = save_path.clone();
            let current_thread = thread::current();
            let thread_name = current_thread.name().unwrap_or("?");
            let builder = thread::Builder::new().name(format!("{}-save", thread_name));

            builder
                .spawn(move || {
                    let save_path_string = save_path.to_string_lossy().to_string();
                    match file_content.save(&save_path) {
                        Ok(_) => log::info!("saved: `{}`", save_path_string),
                        Err(_) => log::info!("failed to save file: `{}`", save_path_string),
                    }
                })
                .map_err(Error::IOError)?;
        }

        // send response
        let save_path = save_path.to_string_lossy().to_string();
        let response = Response { save_path };
        let response = bincode::serialize(&response).map_err(Error::BincodeError)?;
        writer.write_binary(&response).map_err(Error::IOError)?;
    }
}
