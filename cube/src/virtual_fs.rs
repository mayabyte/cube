use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct VirtualFile {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
}

impl VirtualFile {
    pub fn with_path(&self, new_path: impl AsRef<Path>) -> Self {
        VirtualFile {
            path: new_path.as_ref().to_path_buf(),
            bytes: self.bytes.clone(),
        }
    }

    pub fn set_path(&mut self, new_path: impl AsRef<Path>) {
        self.path = new_path.as_ref().to_owned()
    }
}
