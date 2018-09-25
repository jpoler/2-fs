use std::io::{self, SeekFrom};

use traits;
use util::align_down;
use vfat::{Cluster, Metadata, Shared, VFat};

#[derive(Debug)]
pub struct File {
    vfat: Shared<VFat>,
    start: Cluster,
    name: String,
    metadata: Metadata,
    pos: usize,
}

impl File {
    pub fn new(vfat: Shared<VFat>, start: Cluster, name: String, metadata: Metadata) -> File {
        File {
            vfat,
            start,
            name,
            metadata,
            pos: 0,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    fn size(&self) -> u64 {
        self.metadata.size
    }
}

/// Trait implemented by files in the file system.
impl traits::File for File {
    /// Writes any buffered data to disk.
    fn sync(&mut self) -> io::Result<()> {
        unimplemented!("File::sync(): read-only filesystem")
    }

    /// Returns the size of the file in bytes.
    fn size(&self) -> u64 {
        self.size()
    }
}

impl io::Read for File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos == self.size() as usize {
            return Ok(0);
        }

        let mut vfat = self.vfat.borrow_mut();
        let cluster_size_bytes = vfat.cluster_size_bytes();
        let pos_within_cluster = self.pos - align_down(self.pos, cluster_size_bytes);
        let file_end_relative = self.size() as usize - self.pos;
        let read_start_relative = Cluster::from((self.pos / cluster_size_bytes) as u32);

        let max = buf.len();
        let mut inner_buf = vec![];
        let mut n = vfat.read_chain(self.start + read_start_relative, &mut inner_buf, Some(max))?;

        n -= pos_within_cluster;
        n = *vec![n, max, file_end_relative].iter().min().unwrap();
        self.pos += n;

        buf[..n].copy_from_slice(&inner_buf[pos_within_cluster..pos_within_cluster + n]);
        Ok(n)
    }
}

impl io::Write for File {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        unimplemented!("File::Write: read-only filesystem")
    }

    fn flush(&mut self) -> io::Result<()> {
        unimplemented!("File::flush(): read-only filesystem")
    }
}

impl io::Seek for File {
    /// Seek to offset `pos` in the file.
    ///
    /// A seek to the end of the file is allowed. A seek _beyond_ the end of the
    /// file returns an `InvalidInput` error.
    ///
    /// If the seek operation completes successfully, this method returns the
    /// new position from the start of the stream. That position can be used
    /// later with SeekFrom::Start.
    ///
    /// # Errors
    ///
    /// Seeking before the start of a file or beyond the end of the file results
    /// in an `InvalidInput` error.
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let pos = match pos {
            SeekFrom::Start(pos) => pos as i64,
            SeekFrom::End(pos) => self.size() as i64 + pos,
            SeekFrom::Current(pos) => self.pos as i64 + pos,
        };

        if pos < 0 || pos >= self.size() as i64 {
            return Err(io::ErrorKind::InvalidInput.into());
        }

        self.pos = pos as usize;
        Ok(self.pos as u64)
    }
}
