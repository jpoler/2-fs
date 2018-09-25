use std::collections::hash_map::{Entry, HashMap};
use std::{fmt, io};

use traits::BlockDevice;

#[derive(Debug)]
struct CacheEntry {
    data: Vec<u8>,
    dirty: bool,
}

#[derive(Debug, Clone)]
pub struct Partition {
    /// The physical sector where the partition begins.
    pub start: u64,
    /// The size, in bytes, of a logical sector in the partition.
    pub sector_size: u64,
}

pub struct CachedDevice {
    device: Box<BlockDevice>,
    cache: HashMap<u64, CacheEntry>,
    partition: Partition,
}

impl CachedDevice {
    /// Creates a new `CachedDevice` that transparently caches sectors from
    /// `device` and maps physical sectors to logical sectors inside of
    /// `partition`. All reads and writes from `CacheDevice` are performed on
    /// in-memory caches.
    ///
    /// The `partition` parameter determines the size of a logical sector and
    /// where logical sectors begin. An access to a sector `n` _before_
    /// `partition.start` is made to physical sector `n`. Cached sectors before
    /// `partition.start` are the size of a physical sector. An access to a
    /// sector `n` at or after `partition.start` is made to the _logical_ sector
    /// `n - partition.start`. Cached sectors at or after `partition.start` are
    /// the size of a logical sector, `partition.sector_size`.
    ///
    /// `partition.sector_size` must be an integer multiple of
    /// `device.sector_size()`.
    ///
    /// # Panics
    ///
    /// Panics if the partition's sector size is < the device's sector size.
    pub fn new<T>(device: T, partition: Partition) -> CachedDevice
    where
        T: BlockDevice + 'static,
    {
        assert!(partition.sector_size >= device.sector_size());

        CachedDevice {
            device: Box::new(device),
            cache: HashMap::new(),
            partition: partition,
        }
    }

    /// Maps a user's request for a sector `virt` to the physical sector and
    /// number of physical sectors required to access `virt`.
    fn virtual_to_physical(&self, virt: u64) -> (u64, u64) {
        if self.device.sector_size() == self.partition.sector_size {
            (virt, 1)
        } else if virt < self.partition.start {
            (virt, 1)
        } else {
            let factor = self.partition.sector_size / self.device.sector_size();
            let logical_offset = virt - self.partition.start;
            let physical_offset = logical_offset * factor;
            let physical_sector = self.partition.start + physical_offset;
            (physical_sector, factor)
        }
    }

    /// Returns a mutable reference to the cached sector `sector`. If the sector
    /// is not already cached, the sector is first read from the disk.
    ///
    /// The sector is marked dirty as a result of calling this method as it is
    /// presumed that the sector will be written to. If this is not intended,
    /// use `get()` instead.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an error reading the sector from the disk.
    pub fn get_mut(&mut self, sector: u64) -> io::Result<&mut [u8]> {
        let sector_size = self.device.sector_size() as usize;
        match self.cache.entry(sector) {
            Entry::Occupied(occupied) => {
                let cache_entry = occupied.into_mut();
                cache_entry.dirty = true;
                Ok(&mut cache_entry.data[..sector_size])
            }
            Entry::Vacant(vacant) => {
                let mut data = Vec::with_capacity(sector_size);
                self.device.read_sector(sector, &mut data[..sector_size])?;
                let cache_entry = vacant.insert(CacheEntry { data, dirty: true });
                Ok(&mut cache_entry.data[..sector_size])
            }
        }
    }

    /// Returns a reference to the cached sector `sector`. If the sector is not
    /// already cached, the sector is first read from the disk.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an error reading the sector from the disk.
    pub fn get(&mut self, sector: u64) -> io::Result<&[u8]> {
        let sector_size = self.device.sector_size() as usize;
        match self.cache.entry(sector) {
            Entry::Occupied(occupied) => {
                let cache_entry = occupied.into_mut();
                Ok(&cache_entry.data[..sector_size])
            }
            Entry::Vacant(vacant) => {
                let mut data = vec![];
                self.device.read_all_sector(sector, &mut data)?;
                let cache_entry = vacant.insert(CacheEntry { data, dirty: false });
                Ok(&cache_entry.data[..sector_size])
            }
        }
    }

    pub fn get_logical(
        &mut self,
        sector: u64,
        logical_offset: usize,
    ) -> io::Result<(usize, &[u8])> {
        let (sector, factor) = self.virtual_to_physical(sector);
        let factor = factor as usize;
        let sector_size = self.device.sector_size() as usize;
        let sector_offset = logical_offset / sector_size;
        if sector_offset > factor {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid logical offset",
            ));
        }

        let physical_offset = logical_offset % sector_size;
        let sector = self.get(sector + sector_offset as u64)?;
        Ok((physical_offset, sector))
    }
}

impl BlockDevice for CachedDevice {
    fn logical_sector_size(&self) -> u64 {
        self.partition.sector_size
    }

    fn read_sector(&mut self, n: u64, buf: &mut [u8]) -> io::Result<usize> {
        let (n, factor) = self.virtual_to_physical(n);
        let factor = factor as usize;
        let sector_size = self.device.sector_size() as usize;
        if buf.len() < sector_size * factor {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "insufficient buffer capacity",
            ));
        }
        for (i, chunk) in buf[..(sector_size * factor)]
            .chunks_mut(sector_size)
            .enumerate()
        {
            let sector = self.get(n + i as u64)?;
            chunk.copy_from_slice(&sector);
        }

        Ok(sector_size * factor)
    }

    fn write_sector(&mut self, n: u64, buf: &[u8]) -> io::Result<usize> {
        let (n, factor) = self.virtual_to_physical(n);
        let factor = factor as usize;
        let sector_size = self.device.sector_size() as usize;
        if buf.len() < sector_size * factor {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "insufficient buffer capacity",
            ));
        }

        for (i, chunk) in buf[..(sector_size * factor)]
            .chunks(sector_size)
            .enumerate()
        {
            let sector = self.get_mut(n + i as u64)?;
            sector.copy_from_slice(&chunk);
        }

        Ok(sector_size * factor)
    }
}

impl fmt::Debug for CachedDevice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CachedDevice")
            .field("device", &"<block device>")
            .field("cache", &self.cache)
            .finish()
    }
}
