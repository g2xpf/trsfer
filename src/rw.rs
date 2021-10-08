use std::io;
use std::io::{Read, Write};

use indicatif::ProgressBar;
use serde::{Deserialize, Serialize};

pub type ReadBytes = f32;

pub trait BinaryRead: Read {
    fn read_binary(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        // read binary data length
        let mut len_buf = [0u8; 8];
        self.read_exact(&mut len_buf)?;
        let data_size = usize::from_le_bytes(len_buf);
        let mut data_buf = vec![0u8; data_size];
        self.read_exact(&mut data_buf)?;
        buf.extend_from_slice(&data_buf);
        Ok(data_size)
    }

    fn read_deserialize<'a, T>(&mut self, buf: &'a mut Vec<u8>) -> io::Result<T>
    where
        T: Deserialize<'a>,
    {
        self.read_binary(buf)?;
        let t = bincode::deserialize(buf).unwrap();
        Ok(t)
    }

    fn read_deserialize_with_progress<'a, T>(
        &mut self,
        buf: &'a mut Vec<u8>,
        progress_bar: &ProgressBar,
    ) -> io::Result<T>
    where
        T: Deserialize<'a>,
    {
        self.read_binary_with_progress(buf, progress_bar)?;
        let t = bincode::deserialize(buf).unwrap();
        Ok(t)
    }

    fn read_with_progress(
        &mut self,
        buf: &mut Vec<u8>,
        len: usize,
        progress_bar: &ProgressBar,
    ) -> io::Result<usize> {
        progress_bar.reset();
        progress_bar.set_length(1 + len as u64);

        *buf = vec![0u8; len];
        let mut read = 0usize;
        let chunk_size = 4096;

        while read < len {
            let chunk_size = chunk_size.min(len - read);
            self.read_exact(&mut buf[read..read + chunk_size])?;
            read += chunk_size;
            progress_bar.set_position(read as u64);
        }
        assert_eq!(len, read);

        Ok(read)
    }

    fn read_binary_with_progress(
        &mut self,
        buf: &mut Vec<u8>,
        progress_bar: &ProgressBar,
    ) -> io::Result<usize> {
        let mut len_buf = [0u8; 8];
        self.read_exact(&mut len_buf)?;
        let data_size = usize::from_le_bytes(len_buf);

        self.read_with_progress(buf, data_size, progress_bar)
    }
}

impl<R: Read> BinaryRead for R {}

pub trait BinaryWrite: Write {
    fn write_binary(&mut self, buf: &[u8]) -> io::Result<usize> {
        send_binary_size(self, buf)?;

        // write binary data
        let len = self.write(buf)?;
        self.flush()?;

        Ok(len)
    }

    fn write_serialize<T>(&mut self, t: &T) -> io::Result<usize>
    where
        T: Serialize,
    {
        let buf = bincode::serialize(t).unwrap();
        self.write_binary(&buf)
    }

    fn write_with_progress(
        &mut self,
        buf: &[u8],
        len: usize,
        progress_bar: &ProgressBar,
    ) -> io::Result<usize> {
        progress_bar.reset();
        progress_bar.set_length(1 + len as u64);

        let mut written = 0usize;
        let chunk_size = 4096;

        while written < len {
            let chunk_size = chunk_size.min(len - written);
            self.write_all(&buf[written..written + chunk_size])?;
            written += chunk_size;
            progress_bar.set_position(written as u64);
        }

        assert_eq!(len, written);

        Ok(written)
    }

    fn write_serialize_with_progress<T>(
        &mut self,
        t: &T,
        progress_bar: &ProgressBar,
    ) -> io::Result<usize>
    where
        T: Serialize,
    {
        let buf = bincode::serialize(t).unwrap();
        self.write_binary_with_progress(&buf, progress_bar)
    }

    fn write_binary_with_progress(
        &mut self,
        buf: &[u8],
        progress_bar: &ProgressBar,
    ) -> io::Result<usize> {
        send_binary_size(self, buf)?;

        let data_size = buf.len();
        self.write_with_progress(buf, data_size, progress_bar)
    }
}

fn send_binary_size<W: BinaryWrite + ?Sized>(w: &mut W, buf: &[u8]) -> io::Result<usize> {
    let len = buf.len();

    // write binary length
    let len_bytes = usize::to_le_bytes(len);
    w.write(&len_bytes)
}

impl<W: Write> BinaryWrite for W {}
