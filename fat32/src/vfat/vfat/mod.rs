#[cfg(test)]
mod tests;

use std::cmp::min;
use std::io;
use std::mem::size_of;
use std::path::Path;

use mbr::{BootIndicator, MasterBootRecord, PartitionEntry, PartitionType};
use traits::{BlockDevice, FileSystem};
use util::SliceExt;
use vfat::{BiosParameterBlock, CachedDevice, Partition};
use vfat::{Cluster, Dir, Entry, Error, FatEntry, File, Shared, Status};

#[derive(Debug)]
pub struct VFat {
    device: CachedDevice,
    bytes_per_sector: u64,
    sectors_per_cluster: u64,
    sectors_per_fat: u64,
    fat_start_sector: u64,
    data_start_sector: u64,
    root_dir_cluster: Cluster,
}

impl<'a> VFat {
    pub fn from<T>(mut device: T) -> Result<Shared<VFat>, Error>
    where
        T: BlockDevice + 'static,
    {
        let mbr = MasterBootRecord::from(&mut device)?;
        let partition = mbr
            .table
            .iter()
            .find(
                |partition| match (partition.boot_indicator, partition.partition_type) {
                    (BootIndicator::Active, PartitionType::Fat32Chs) => true,
                    (BootIndicator::Active, PartitionType::Fat32Lba) => true,
                    _ => false,
                },
            ).ok_or(Error::NoBootableFatPartition)?;
        let ebpb = BiosParameterBlock::from(&mut device, partition.relative_sector as u64)?;

        let vfat = VFat::from_inner(device, partition, &ebpb);
        Ok(Shared::new(vfat))
    }

    fn from_inner<T>(device: T, partition: &PartitionEntry, ebpb: &BiosParameterBlock) -> VFat
    where
        T: BlockDevice + 'static,
    {
        let cache_partition = Partition {
            start: partition.relative_sector as u64,
            sector_size: ebpb.bytes_per_sector as u64,
        };
        let vfat = VFat {
            device: CachedDevice::new(device, cache_partition),
            bytes_per_sector: ebpb.bytes_per_sector as u64,
            sectors_per_cluster: ebpb.sectors_per_cluster as u64,
            sectors_per_fat: ebpb.sectors_per_fat as u64,
            fat_start_sector: partition.relative_sector as u64 + ebpb.relative_fat_start_sector(),
            data_start_sector: partition.relative_sector as u64 + ebpb.relative_data_start_sector(),
            root_dir_cluster: Cluster::from(ebpb.root_cluster),
        };

        assert!(vfat.bytes_per_sector % (size_of::<FatEntry>() as u64) == 0);

        vfat
    }

    //  * A method to read from an offset of a cluster into a buffer.
    //
    fn read_cluster(
        &mut self,
        cluster: Cluster,
        offset: usize,
        buf: &mut [u8],
    ) -> io::Result<usize> {
        let cluster_size_bytes = self.cluster_size_bytes();
        assert!(buf.len() >= cluster_size_bytes);
        assert!(offset <= cluster_size_bytes);
        let cluster_sector = self.cluster_sector(&cluster);

        let mut n = 0;
        for (i, mut chunk) in buf[..cluster_size_bytes]
            .chunks_mut(self.bytes_per_sector as usize)
            .enumerate()
        {
            n += self
                .device
                .read_sector(cluster_sector + i as u64, &mut chunk)?;
        }

        Ok(n)
    }

    //  * A method to read all of the clusters chained from a starting cluster
    //    into a vector.
    //
    pub fn read_chain(
        &mut self,
        start: Cluster,
        mut buf: &mut Vec<u8>,
        max: Option<usize>,
    ) -> io::Result<usize> {
        let sectors_per_cluster = self.sectors_per_cluster;

        let entries =
            FatIter::new(self, start).collect::<io::Result<Vec<(Cluster, FatEntry)>>>()?;

        let mut n = 0;
        for (cluster, entry) in entries {
            match max {
                Some(max) if n >= max => break,
                _ => {}
            }

            match entry.status() {
                Status::Data(_) => {
                    let cluster_sector = self.cluster_sector(&cluster);
                    for i in 0..sectors_per_cluster {
                        n += self
                            .device
                            .read_all_sector(cluster_sector + i as u64, &mut buf)?;
                    }
                }
                Status::Eoc(_) => break,
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "invalid cluster chain",
                    ))
                }
            }
        }

        Ok(n)
    }

    //  * A method to return a reference to a `FatEntry` for a cluster where the
    //    reference points directly into a cached sector.
    //
    fn fat_entry(&mut self, cluster: Cluster) -> io::Result<FatEntry> {
        let n = cluster.get();
        let sector = self.fat_entry_sector(n);
        let offset = self.fat_sector_offset(n);
        let buf = self.device.get(sector)?;
        let fat_entries = unsafe { buf.cast::<FatEntry>() };
        Ok(fat_entries[offset])
    }

    pub fn cluster_size_bytes(&self) -> usize {
        (self.bytes_per_sector * self.sectors_per_cluster) as usize
    }

    fn cluster_sector(&self, cluster: &Cluster) -> u64 {
        self.data_start_sector + self.sectors_per_cluster * (cluster.get() as u64 - 2)
    }

    fn fat_entry_sector(&self, n: u32) -> u64 {
        self.fat_start_sector + (n as u64 / self.fats_per_sector())
    }

    fn fat_sector_offset(&self, n: u32) -> usize {
        (n as usize % self.fats_per_sector() as usize)
    }

    fn fats_per_sector(&self) -> u64 {
        self.bytes_per_sector / size_of::<FatEntry>() as u64
    }

    fn fat_size_bytes(&self) -> u64 {
        self.bytes_per_sector * self.sectors_per_fat
    }
}

struct FatIter<'a> {
    vfat: &'a mut VFat,
    current: Option<Cluster>,
}

impl<'a> FatIter<'a> {
    fn new(vfat: &'a mut VFat, cluster: Cluster) -> FatIter {
        FatIter {
            vfat,
            current: Some(cluster),
        }
    }
}

impl<'a> Iterator for FatIter<'a> {
    type Item = io::Result<(Cluster, FatEntry)>;

    fn next(&mut self) -> Option<Self::Item> {
        let cluster = self.current?;
        let result = self.vfat.fat_entry(cluster).map(|entry| {
            match entry.status() {
                Status::Data(next_cluster) => self.current = Some(next_cluster),
                _ => self.current = None,
            }
            (cluster, entry)
        });
        Some(result)
    }
}

impl<'a> FileSystem for &'a Shared<VFat> {
    type File = ::traits::Dummy;
    type Dir = ::traits::Dummy;
    type Entry = ::traits::Dummy;

    fn open<P: AsRef<Path>>(self, path: P) -> io::Result<Self::Entry> {
        unimplemented!("FileSystem::open()")
    }

    fn create_file<P: AsRef<Path>>(self, _path: P) -> io::Result<Self::File> {
        unimplemented!("read only file system")
    }

    fn create_dir<P>(self, _path: P, _parents: bool) -> io::Result<Self::Dir>
    where
        P: AsRef<Path>,
    {
        unimplemented!("read only file system")
    }

    fn rename<P, Q>(self, _from: P, _to: Q) -> io::Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        unimplemented!("read only file system")
    }

    fn remove<P: AsRef<Path>>(self, _path: P, _children: bool) -> io::Result<()> {
        unimplemented!("read only file system")
    }
}
