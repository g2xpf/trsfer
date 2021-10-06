use std::io;
use std::io::{Read, Write};

pub trait BinaryRead: Read {
    fn read_binary(&mut self, buf: &mut Vec<u8>) -> io::Result<usize>;
}

pub struct BinaryReader<R> {
    inner: R,
}

impl<R: Read> BinaryReader<R> {
    pub fn new(inner: R) -> Self {
        BinaryReader { inner }
    }
}

impl<R> Read for BinaryReader<R>
where
    R: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<R> BinaryRead for BinaryReader<R>
where
    R: Read,
{
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
}

pub trait BinaryWrite: Write {
    fn write_binary(&mut self, buf: &[u8]) -> io::Result<usize>;
}

pub struct BinaryWriter<W> {
    inner: W,
}

impl<W: Write> BinaryWriter<W> {
    pub fn new(inner: W) -> Self {
        BinaryWriter { inner }
    }
}

impl<W: Write> Write for BinaryWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<W: Write> BinaryWrite for BinaryWriter<W> {
    fn write_binary(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = buf.len();

        // write binary length
        let len_bytes = usize::to_le_bytes(len);
        self.write(&len_bytes)?;

        // write binary data
        self.write(buf)?;
        self.flush()?;

        Ok(len)
    }
}
