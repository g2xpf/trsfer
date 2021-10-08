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
        reader.read_binary(&mut buf)?;
        let setting = bincode::deserialize::<TrsferSetting>(&buf).unwrap();

        let mut buf = Vec::new();
        reader.read_binary(&mut buf)?;
        let file_metadata = bincode::deserialize::<FileMetadata>(&buf).unwrap();

        // set progress bar message
        let mut buf = Vec::new();
        reader.read_binary(&mut buf)?;

        let file_content = FileContent(buf);

        let save_path = output_path.join(&file_metadata.path_buf);

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
