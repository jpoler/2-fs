use traits;
use vfat::{Cluster, Dir, File, Metadata};

// TODO: You may need to change this definition.
#[derive(Debug)]
pub enum Entry {
    File(File),
    Dir(Dir),
}

// TODO: Implement any useful helper methods on `Entry`.

// FIXME: Implement `traits::Entry` for `Entry`.

impl traits::Entry for Entry {
    type File = File;
    type Dir = Dir;
    type Metadata = Metadata;

    /// The name of the file or directory corresponding to this entry.
    fn name(&self) -> &str {
        unimplemented!()
    }

    /// The metadata associated with the entry.
    fn metadata(&self) -> &Self::Metadata {
        unimplemented!()
    }

    /// If `self` is a file, returns `Some` of a reference to the file.
    /// Otherwise returns `None`.
    fn as_file(&self) -> Option<&Self::File> {
        unimplemented!()
    }

    /// If `self` is a directory, returns `Some` of a reference to the
    /// directory. Otherwise returns `None`.
    fn as_dir(&self) -> Option<&Self::Dir> {
        unimplemented!()
    }

    /// If `self` is a file, returns `Some` of the file. Otherwise returns
    /// `None`.
    fn into_file(self) -> Option<Self::File> {
        unimplemented!()
    }

    /// If `self` is a directory, returns `Some` of the directory. Otherwise
    /// returns `None`.
    fn into_dir(self) -> Option<Self::Dir> {
        unimplemented!()
    }
}
