use std::fmt;
use std::mem::transmute;

use traits::BlockDevice;
use vfat::Error;

#[repr(C, packed)]
pub struct BiosParameterBlock {
    pub _asm: [u8; 3],
    pub oem_id: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub fats: u8,
    pub max_dir_entries: u16,
    pub logical_sectors_small: u16,
    pub fat_id: u8,
    pub _deprecated_sectors_per_fat: u16,
    pub _sectors_per_track: u16,
    pub _heads: u16,
    pub hidden_sectors: u32,
    pub logical_sectors_large: u32,
    pub sectors_per_fat: u32,
    pub flags: u16,
    pub fat_version_number_minor: u8,
    pub fat_version_number_major: u8,
    pub root_cluster: u32,
    pub fs_info_sector: u16,
    pub backup_boot_sector: u16,
    pub _reserved: [u8; 12],
    pub drive_number: u8,
    pub _windows_nt_flags: u8,
    pub signature: u8,
    pub _volume_id: u32,
    pub volume_label: [u8; 11],
    pub system_id: [u8; 8],
    pub boot_code: [u8; 420],
    pub partition_signature: [u8; 2],
}

impl BiosParameterBlock {
    fn check_signature(&self) -> Result<(), Error> {
        if self.partition_signature[0] != 0xAA || self.partition_signature[1] != 0x55 {
            Err(Error::BadSignature)
        } else {
            Ok(())
        }
    }

    /// Reads the FAT32 extended BIOS parameter block from sector `sector` of
    /// device `device`.
    ///
    /// # Errors
    ///
    /// If the EBPB signature is invalid, returns an error of `BadSignature`.
    pub fn from<T: BlockDevice>(mut device: T, sector: u64) -> Result<BiosParameterBlock, Error> {
        let mut buf: [u8; 512] = [0; 512];
        device.read_sector(sector, &mut buf)?;
        let ebpb = unsafe { transmute::<[u8; 512], BiosParameterBlock>(buf) };

        ebpb.check_signature()?;
        Ok(ebpb)
    }
}

impl fmt::Debug for BiosParameterBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BiosParameterBlock")
            .field("oem_id", &self.oem_id)
            .field("bytes_per_sector", &self.bytes_per_sector)
            .field("reserved_sectors", &self.reserved_sectors)
            .field("fats", &self.fats)
            .field("max_dir_entries", &self.max_dir_entries)
            .field("logical_sectors_small", &self.logical_sectors_small)
            .field("fat_id", &self.fat_id)
            .field("hidden_sectors", &self.hidden_sectors)
            .field("logical_sectors_large", &self.logical_sectors_large)
            .field("sectors_per_fat", &self.sectors_per_fat)
            .field("flags", &self.flags)
            .field("fat_version_number_minor", &self.fat_version_number_minor)
            .field("fat_version_number_major", &self.fat_version_number_major)
            .field("root_cluster", &self.root_cluster)
            .field("fs_info_sector", &self.fs_info_sector)
            .field("backup_boot_sector", &self.backup_boot_sector)
            .field("drive_number", &self.drive_number)
            .field("signature", &self.signature)
            .field("volume_label", &self.volume_label)
            .field("system_id", &self.system_id)
            .field("partition_signature", &self.partition_signature)
            .finish()
    }
}
