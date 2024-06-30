use crate::virtual_fs::VirtualFile;
use gc_gcm::{DirEntry, GcmError, GcmFile};
use std::{
    error::Error,
    fmt::Display,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

pub fn extract_iso<P: AsRef<Path>>(iso_path: P) -> Result<Vec<VirtualFile>, IsoError> {
    let iso_path = iso_path.as_ref();
    let iso = GcmFile::open(iso_path)?;
    let all_files = traverse_filesystem(&iso);
    let mut iso_reader = BufReader::new(File::open(iso_path)?);
    all_files
        .into_iter()
        .map(|vgf| vgf.read(&mut iso_reader).map_err(Into::into))
        .collect()
}

#[derive(Debug)]
struct VirtualGcmFile<'a> {
    pub path: PathBuf,
    pub entry: DirEntry<'a>,
}

impl<'a> VirtualGcmFile<'a> {
    fn wrap(entry: DirEntry<'a>, path: PathBuf) -> Self {
        Self { path, entry }
    }

    fn read(self, iso_reader: &mut BufReader<File>) -> std::io::Result<VirtualFile> {
        let file_location = self.entry.as_file().unwrap();
        let mut data = vec![0u8; file_location.size as usize];
        iso_reader.seek(SeekFrom::Start(file_location.offset as u64))?;
        iso_reader.read_exact(&mut data)?;
        Ok(VirtualFile {
            path: self.path,
            bytes: data,
        })
    }
}

fn traverse_filesystem(iso: &GcmFile) -> Vec<VirtualGcmFile<'_>> {
    traverse_fs_recursive(
        iso.filesystem
            .iter_root()
            .map(|e| VirtualGcmFile::wrap(e, PathBuf::new()))
            .collect(),
    )
}

fn traverse_fs_recursive(entries: Vec<VirtualGcmFile<'_>>) -> Vec<VirtualGcmFile<'_>> {
    let (mut files, directories): (Vec<_>, Vec<_>) =
        entries.into_iter().partition(|e| e.entry.is_file());
    files
        .iter_mut()
        .for_each(|f| f.path.push(f.entry.entry_name()));
    files.extend(directories.into_iter().flat_map(|mut d| {
        d.path.push(d.entry.entry_name());
        traverse_fs_recursive(
            d.entry
                .iter_dir()
                .unwrap()
                .map(|e| VirtualGcmFile::wrap(e, d.path.clone()))
                .collect(),
        )
    }));
    files
}

#[derive(Debug)]
pub struct IsoError(GcmError);

impl Error for IsoError {}

impl Display for IsoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            GcmError::ParseError(e) => e.fmt(f),
            GcmError::IoError(e) => e.fmt(f),
        }
    }
}

impl From<GcmError> for IsoError {
    fn from(value: GcmError) -> Self {
        IsoError(value)
    }
}

impl From<std::io::Error> for IsoError {
    fn from(value: std::io::Error) -> Self {
        IsoError(GcmError::IoError(value))
    }
}
