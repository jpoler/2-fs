use std::mem::transmute;
use std::{fmt, io};

use traits::BlockDevice;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BootIndicator {
    No = 0x00,
    Active = 0x80,
    Unknown,
}

impl Default for BootIndicator {
    fn default() -> Self {
        BootIndicator::No
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PartitionType {
    Fat32Chs = 0x0b,
    Fat32Lba = 0x0c,
    Unsupported,
}

impl Default for PartitionType {
    fn default() -> Self {
        PartitionType::Unsupported
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct CHS {
    // we don't care about the format, so bothering with bitfields isn't worth
    // it
    pub _chs: [u8; 3],
}

#[repr(C, packed)]
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PartitionEntry {
    pub boot_indicator: BootIndicator,
    pub _start_chs: CHS,
    pub partition_type: PartitionType,
    pub _end_chs: CHS,
    pub relative_sector: u32,
    pub sectors: u32,
}

/// The master boot record (MBR).
#[repr(C, packed)]
pub struct MasterBootRecord {
    bootstrap: [u8; 436],
    pub id: [u8; 10],
    pub table: [PartitionEntry; 4],
    signature: [u8; 2],
}

#[derive(Debug)]
pub enum Error {
    /// There was an I/O error while reading the MBR.
    Io(io::Error),
    /// Partiion `.0` (0-indexed) contains an invalid or unknown boot indicator.
    UnknownBootIndicator(u8),
    /// UnsupportedPartitionType contains the unsupported partition type specifier.
    UnsupportedPartitionType(u8),
    /// The MBR magic signature was invalid.
    BadSignature,
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Error {
        Error::Io(error)
    }
}

impl MasterBootRecord {
    fn check_boot_indicators(&self) -> Result<(), Error> {
        for (i, entry) in self.table.iter().enumerate() {
            match entry.boot_indicator {
                BootIndicator::No => {}
                BootIndicator::Active => {}
                _ => return Err(Error::UnknownBootIndicator(i as u8)),
            }
        }

        Ok(())
    }

    fn check_signature(&self) -> Result<(), Error> {
        if self.signature[0] != 0x55 || self.signature[1] != 0xAA {
            Err(Error::BadSignature)
        } else {
            Ok(())
        }
    }

    /// Reads and returns the master boot record (MBR) from `device`.
    ///
    /// # Errors
    ///
    /// Returns `BadSignature` if the MBR contains an invalid magic signature.
    /// Returns `UnknownBootIndicator(n)` if partition `n` contains an invalid
    /// boot indicator. Returns `Io(err)` if the I/O error `err` occured while
    /// reading the MBR.
    pub fn from<T: BlockDevice>(mut device: T) -> Result<MasterBootRecord, Error> {
        let mut buf: [u8; 512] = [0; 512];
        device.read_sector(0, &mut buf)?;
        let mbr = unsafe { transmute::<[u8; 512], MasterBootRecord>(buf) };
        mbr.check_boot_indicators()?;
        mbr.check_signature()?;
        Ok(mbr)
    }
}

impl fmt::Debug for MasterBootRecord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut debug_struct = f.debug_struct("MasterBootRecord");
        debug_struct.field("id", &self.id);
        for (i, entry) in self.table.iter().enumerate() {
            debug_struct.field(&format!("entry {}", i), &entry);
        }

        debug_struct.finish()
    }
}
