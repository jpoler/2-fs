use std::borrow::Cow;
use std::char::{decode_utf16, REPLACEMENT_CHARACTER};
use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::str;

use traits::{self, Dir as DirTrait, Entry as EntryTrait};
use util::VecExt;
use vfat::{Attributes, Date, Metadata, Time, Timestamp};
use vfat::{Cluster, Entry, File, Shared, VFat};

#[derive(Debug)]
pub struct Dir {
    vfat: Shared<VFat>,
    start: Cluster,
    name: String,
    metadata: Metadata,
}

impl Dir {
    pub fn new(vfat: Shared<VFat>, start: Cluster, name: String, metadata: Metadata) -> Dir {
        Dir {
            vfat,
            start,
            name,
            metadata,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Finds the entry named `name` in `self` and returns it. Comparison is
    /// case-insensitive.
    ///
    /// # Errors
    ///
    /// If no entry with name `name` exists in `self`, an error of `NotFound` is
    /// returned.
    ///
    /// If `name` contains invalid UTF-8 characters, an error of `InvalidInput`
    /// is returned.
    pub fn find<P: AsRef<OsStr>>(&self, name: P) -> io::Result<Entry> {
        let name = name.as_ref().to_str().ok_or(io::Error::new(
            io::ErrorKind::InvalidInput,
            "name is not valid utf-8",
        ))?;

        let entry = self
            .entries()?
            .find(|entry| entry.name().eq_ignore_ascii_case(name.as_ref()))
            .ok_or(io::Error::new(
                io::ErrorKind::NotFound,
                format!("{}: not found", name),
            ))?;

        Ok(entry)
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatRegularDirEntry {
    name: [u8; 8],
    extension: [u8; 3],
    attributes: u8,
    _nt_reserved: u8,
    _created_time_tenths_second: u8,
    created_time: u16,
    created_date: u16,
    accessed_date: u16,
    cluster_high: u16,
    modified_time: u16,
    modified_date: u16,
    cluster_low: u16,
    size: u32,
}

impl fmt::Debug for VFatRegularDirEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("VFatRegularDirEntry")
            .field("name", &self.name())
            .field("attributes", &self.attributes)
            .field("created_time", &self.created_time)
            .field("created_date", &self.created_date)
            .field("accessed_date", &self.accessed_date)
            .field("modified_time", &self.modified_time)
            .field("modified_date", &self.modified_date)
            .field("cluster", &self.cluster())
            .finish()
    }
}

impl VFatRegularDirEntry {
    fn cluster(&self) -> Cluster {
        Cluster::from(((self.cluster_high as u32) << 16) | (self.cluster_low as u32))
    }

    fn created(&self) -> Timestamp {
        let date = Date::from_raw(self.created_date);
        let time = Time::from_raw(self.created_time);
        Timestamp::new(date, time)
    }

    fn accessed(&self) -> Timestamp {
        let date = Date::from_raw(self.accessed_date);
        Timestamp::new(date, Default::default())
    }

    fn modified(&self) -> Timestamp {
        let date = Date::from_raw(self.modified_date);
        let time = Time::from_raw(self.modified_time);
        Timestamp::new(date, time)
    }

    fn attributes(&self) -> Attributes {
        Attributes::from_raw(self.attributes)
    }

    fn size(&self) -> u64 {
        self.size as u64
    }

    fn metadata(&self) -> Metadata {
        let attributes = self.attributes();
        let created = self.created();
        let accessed = self.accessed();
        let modified = self.modified();
        let size = self.size();
        Metadata {
            attributes,
            created,
            accessed,
            modified,
            size,
        }
    }

    fn name(&self) -> Option<String> {
        let &name_stop = &self.name[..]
            .iter()
            .position(|&c| c == 0x00 || c == b' ')
            .unwrap_or(self.name.len());
        let &ext_stop = &self.extension[..]
            .iter()
            .position(|&c| c == 0x00 || c == b' ')
            .unwrap_or(self.extension.len());
        let name = str::from_utf8(&self.name[..name_stop]).ok()?;
        let extension = str::from_utf8(&self.extension[..ext_stop]).ok()?;

        if name == "" {
            return None;
        }

        if extension != "" {
            Some(format!("{}.{}", name, extension))
        } else {
            Some(format!("{}", name))
        }
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct VFatLfnDirEntry {
    seqno: u8,
    name_1: [u16; 5],
    attributes: u8,
    _reserved_1: u8,
    dos_checksum: u8,
    name_2: [u16; 6],
    _reserved_2: [u8; 2],
    name_3: [u16; 2],
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatUnknownDirEntry {
    _unknown_1: [u8; 11],
    attributes: u8,
    _unknown_2: [u8; 20],
}

pub union VFatDirEntry {
    unknown: VFatUnknownDirEntry,
    regular: VFatRegularDirEntry,
    long_filename: VFatLfnDirEntry,
}

impl From<VFatRegularDirEntry> for VFatEntry {
    fn from(regular: VFatRegularDirEntry) -> VFatEntry {
        VFatEntry::Regular(regular)
    }
}

impl From<VFatLfnDirEntry> for VFatEntry {
    fn from(lfn: VFatLfnDirEntry) -> VFatEntry {
        VFatEntry::Lfn(lfn)
    }
}

impl<'a> From<&'a VFatDirEntry> for VFatEntry {
    fn from(dir_entry: &'a VFatDirEntry) -> VFatEntry {
        let attributes = unsafe { dir_entry.unknown.attributes };
        let attributes = Attributes::from_raw(attributes);

        unsafe {
            match (attributes.lfn(), dir_entry) {
                (true, &VFatDirEntry { long_filename }) => long_filename.into(),
                (false, &VFatDirEntry { regular }) => regular.into(),
            }
        }
    }
}

#[derive(Debug)]
enum VFatEntry {
    Regular(VFatRegularDirEntry),
    Lfn(VFatLfnDirEntry),
}

impl VFatEntry {
    fn regular(&self) -> Option<&VFatRegularDirEntry> {
        if let &VFatEntry::Regular(ref reg) = self {
            Some(reg)
        } else {
            None
        }
    }

    fn lfn(&self) -> Option<&VFatLfnDirEntry> {
        if let &VFatEntry::Lfn(ref lfn) = self {
            Some(lfn)
        } else {
            None
        }
    }
}

impl traits::Dir for Dir {
    /// The type of entry stored in this directory.
    type Entry = Entry;

    /// An type that is an iterator over the entries in this directory.
    type Iter = DirIter;

    /// Returns an interator over the entries in this directory.
    fn entries(&self) -> io::Result<Self::Iter> {
        let mut vfat = self.vfat.borrow_mut();
        let mut buf = vec![];

        vfat.read_chain(self.start, &mut buf, None)?;

        let buf = unsafe { buf.cast::<VFatDirEntry>() };

        Ok(DirIter::new(self.vfat.clone(), buf))
    }
}

pub struct DirIter {
    vfat: Shared<VFat>,
    buf: Vec<VFatDirEntry>,
    current: usize,
}

impl DirIter {
    fn new(vfat: Shared<VFat>, buf: Vec<VFatDirEntry>) -> DirIter {
        DirIter {
            vfat,
            buf,
            current: 0,
        }
    }

    fn name_from_lfn(&self, lfn_start: usize, lfn_stop: usize) -> Option<String> {
        let mut entries: Vec<VFatEntry> = (&self.buf[lfn_start..lfn_stop])
            .iter()
            .rev()
            .map(|entry| entry.into())
            .take_while(|entry| {
                if let &VFatEntry::Lfn(_) = entry {
                    true
                } else {
                    false
                }
            }).filter(|entry| match entry {
                &VFatEntry::Lfn(ref lfn) if lfn.seqno != 0xe5 => true,
                _ => false,
            }).collect();

        entries.sort_by_key(|entry| match entry {
            &VFatEntry::Regular(_) => 0,
            &VFatEntry::Lfn(lfn) => lfn.seqno,
        });

        let mut name: Vec<u16> = vec![];
        for (i, entry) in entries.iter().enumerate() {
            println!("i: {}, entry: {:?}", i, entry);
            let lfn = if let &VFatEntry::Lfn(lfn) = entry {
                lfn
            } else {
                println!("not lfn");
                return None;
            };

            println!("seqno: {:x}", lfn.seqno);

            // if lfn.seqno != i as u8 + 1 {
            //     println!("wrong seqno i: {}, seqno: {}", i, lfn.seqno);
            //     return None;
            // }
            name.extend(lfn.name_1.iter());
            name.extend(lfn.name_2.iter());
            name.extend(lfn.name_3.iter());
        }

        println!("name before: {:?}", name);

        let end = name
            .iter()
            .position(|&c| c == 0x0000u16)
            .unwrap_or(name.len());

        // TODO: figure out whether FAT32 handles unpaired surrogates or should
        // error. For the time being I'm just going to replace them.
        let s = decode_utf16((&name[..end]).iter().cloned())
            .map(|c| c.unwrap_or(REPLACEMENT_CHARACTER))
            .collect::<String>();

        if s == "" {
            None
        } else {
            Some(s)
        }
    }
}

// TODO: just ensure that this won't read into garbage data past valid dir
// entries.
impl Iterator for DirIter {
    type Item = Entry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.buf.len() {
            return None;
        }

        let &(reg_index, ref reg) = &self.buf[self.current..]
            .iter()
            .enumerate()
            .map(|(i, union_entry)| (self.current + i, union_entry.into()))
            .find(|&(_, ref entry)| match entry {
                &VFatEntry::Regular(_) => true,
                &VFatEntry::Lfn(_) => false,
            })?;

        let reg = reg.regular()?;

        println!("index: {}, regular dir entry: {:?}", reg_index, reg);

        let name = if self.current < reg_index {
            self.name_from_lfn(self.current, reg_index)
        } else {
            reg.name()
        }?;

        self.current = reg_index + 1;

        let metadata = reg.metadata();
        let start = reg.cluster();
        let vfat = self.vfat.clone();

        if metadata.attributes.directory() {
            Some(Entry::Dir(Dir::new(vfat, start, name, metadata)))
        } else {
            Some(Entry::File(File::new(vfat, start, name, metadata)))
        }
    }
}
