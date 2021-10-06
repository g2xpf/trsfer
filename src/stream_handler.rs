use std::fs::{self, File};
use std::net::TcpStream;

use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;

use super::rw::{BinaryRead, BinaryReader, BinaryWrite, BinaryWriter};
use super::{RawFileEntry, Response};

pub fn handle_stream(stream: TcpStream, output_path: Arc<PathBuf>) -> io::Result<()> {
    let stream_cloned = stream.try_clone()?;
    let mut reader = BinaryReader::new(stream);
    let mut writer = BinaryWriter::new(stream_cloned);

    loop {
        // receive request
        let mut buf = Vec::new();
        reader.read_binary(&mut buf)?;

        let entry = bincode::deserialize::<RawFileEntry>(&buf).unwrap();

        let save_path = output_path.join(&entry.path_buf);

        if !entry.is_dry_run {
            if let Some(parent_path) = save_path.parent() {
                fs::create_dir_all(parent_path)?;
                let mut file = File::create(&save_path)?;
                file.write_all(&entry.raw_data)?;
            }
        }

        log::info!(
            "{}saved `{}`({} bytes)",
            if entry.is_dry_run { "[dry-run] " } else { "" },
            entry.path_buf.to_string_lossy(),
            entry.raw_data.len(),
        );

        // send response
        let save_path = save_path.to_string_lossy().to_string();
        let response = Response { save_path };
        let response = bincode::serialize(&response).unwrap();
        writer.write_binary(&response)?;
    }
}
