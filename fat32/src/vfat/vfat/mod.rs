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
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    sectors_per_fat: u32,
    fat_start_sector: u64,
    data_start_sector: u64,
    root_dir_cluster: Cluster,
}

impl VFat {
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
        VFat {
            device: CachedDevice::new(device, cache_partition),
            bytes_per_sector: ebpb.bytes_per_sector,
            sectors_per_cluster: ebpb.sectors_per_cluster,
            sectors_per_fat: ebpb.sectors_per_fat,
            fat_start_sector: partition.relative_sector as u64,
            data_start_sector: partition.relative_sector as u64 + ebpb.relative_data_start(),
            root_dir_cluster: Cluster::from(ebpb.root_cluster),
        }
    }

    // TODO: The following methods may be useful here:
    //
    //  * A method to read from an offset of a cluster into a buffer.
    //
    //    fn read_cluster(
    //        &mut self,
    //        cluster: Cluster,
    //        offset: usize,
    //        buf: &mut [u8]
    //    ) -> io::Result<usize>;
    //
    //  * A method to read all of the clusters chained from a starting cluster
    //    into a vector.
    //
    //    fn read_chain(
    //        &mut self,
    //        start: Cluster,
    //        buf: &mut Vec<u8>
    //    ) -> io::Result<usize>;
    //
    //  * A method to return a reference to a `FatEntry` for a cluster where the
    //    reference points directly into a cached sector.
    //
    //    fn fat_entry(&mut self, cluster: Cluster) -> io::Result<&FatEntry>;
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
