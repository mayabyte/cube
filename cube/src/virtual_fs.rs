use std::path::PathBuf;

pub struct VirtualFile {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
}
