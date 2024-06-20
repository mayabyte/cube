use std::path::Path;

use crate::virtual_fs::VirtualFile;

/// For turning 'normal' files into GCN file formats
pub trait Encode {
    type Error;
    fn encode<P: AsRef<Path>>(path: P) -> Result<VirtualFile, Self::Error>;
}

/// For turning files in GCN formats into 'normal' file formats
pub trait Decode {
    type Out;
    fn decode<P: AsRef<Path>>(&self) -> Self::Out;
}
