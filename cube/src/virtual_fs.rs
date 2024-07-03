use std::{
    fs::read,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct VirtualFile {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
}

impl VirtualFile {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let path = path.as_ref();
        let bytes = read(path)?;
        Ok(VirtualFile {
            path: path.to_owned(),
            bytes,
        })
    }

    pub fn with_path(self, new_path: impl AsRef<Path>) -> Self {
        VirtualFile {
            path: new_path.as_ref().to_path_buf(),
            bytes: self.bytes,
        }
    }

    pub fn set_path(&mut self, new_path: impl AsRef<Path>) {
        self.path = new_path.as_ref().to_owned()
    }
}
