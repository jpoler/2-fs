use std::fmt;
use traits::{self, Entry as EntryTrait, Metadata as MetadataTrait, Timestamp as TimestampTrait};
use vfat::{Dir, File, Metadata, Timestamp};

#[derive(Debug)]
pub enum Entry {
    File(File),
    Dir(Dir),
}

impl traits::Entry for Entry {
    type File = File;
    type Dir = Dir;
    type Metadata = Metadata;

    /// The name of the file or directory corresponding to this entry.
    fn name(&self) -> &str {
        match self {
            &Entry::File(ref file) => file.name(),
            &Entry::Dir(ref dir) => dir.name(),
        }
    }

    /// The metadata associated with the entry.
    fn metadata(&self) -> &Self::Metadata {
        match self {
            &Entry::File(ref file) => file.metadata(),
            &Entry::Dir(ref dir) => dir.metadata(),
        }
    }

    /// If `self` is a file, returns `Some` of a reference to the file.
    /// Otherwise returns `None`.
    fn as_file(&self) -> Option<&Self::File> {
        match self {
            &Entry::File(ref file) => Some(file),
            _ => None,
        }
    }

    /// If `self` is a directory, returns `Some` of a reference to the
    /// directory. Otherwise returns `None`.
    fn as_dir(&self) -> Option<&Self::Dir> {
        match self {
            &Entry::Dir(ref dir) => Some(dir),
            _ => None,
        }
    }

    /// If `self` is a file, returns `Some` of the file. Otherwise returns
    /// `None`.
    fn into_file(self) -> Option<Self::File> {
        match self {
            Entry::File(file) => Some(file),
            _ => None,
        }
    }

    /// If `self` is a directory, returns `Some` of the directory. Otherwise
    /// returns `None`.
    fn into_dir(self) -> Option<Self::Dir> {
        match self {
            Entry::Dir(dir) => Some(dir),
            _ => None,
        }
    }
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fn write_bool(f: &mut fmt::Formatter, b: bool, c: char) -> ::std::fmt::Result {
            if b {
                write!(f, "{}", c)
            } else {
                write!(f, "-")
            }
        };

        fn write_timestamp<T: TimestampTrait>(f: &mut fmt::Formatter, ts: T) -> ::std::fmt::Result {
            write!(
                f,
                "{:02}/{:02}/{} {:02}:{:02}:{:02} ",
                ts.month(),
                ts.day(),
                ts.year(),
                ts.hour(),
                ts.minute(),
                ts.second()
            )
        };

        let metadata = self.metadata();
        write_bool(f, self.is_dir(), 'd')?;
        write_bool(f, self.is_file(), 'f')?;
        write_bool(f, metadata.read_only(), 'r')?;
        write_bool(f, metadata.hidden(), 'h')?;

        write!(f, "\t")?;

        write_timestamp(f, metadata.created())?;
        write_timestamp(f, metadata.modified())?;
        write_timestamp(f, metadata.accessed())?;

        write!(f, "\t")?;

        write!(f, "{}", self.name())
    }
}
