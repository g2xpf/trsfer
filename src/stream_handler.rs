use std::net::TcpStream;

use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use super::rw::{BinaryRead, BinaryWrite};
use super::{FileContent, FileMetadata, Response, TrsferSetting};

pub fn handle_stream(stream: TcpStream, output_path: Arc<PathBuf>) -> io::Result<()> {
    let stream_cloned = stream.try_clone()?;
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
        reader.read_binary(&mut buf)?;
        let file_content = FileContent(buf);

        if !setting.is_dry_run {
            let save_path = save_path.clone();
            thread::spawn(move || {
                file_content.save(&save_path).unwrap();
                log::info!("{}", save_path.to_string_lossy().to_string());
            });
        }

        // send response
        let save_path = save_path.to_string_lossy().to_string();
        let response = Response { save_path };
        let response = bincode::serialize(&response).unwrap();
        writer.write_binary(&response)?;
    }
}
