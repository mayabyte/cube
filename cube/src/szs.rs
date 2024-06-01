use crate::rarc::Rarc;
use std::{io::Cursor, path::PathBuf};
use yaz0::{Error as Yaz0Error, Yaz0Archive};

/// Extracts an (optionally Yaz0 compressed) SZS archive into a list of files with
/// their respective paths and raw contents.
pub fn extract_szs(data: Vec<u8>) -> Result<Vec<(PathBuf, Vec<u8>)>, Yaz0Error> {
    let arc = if &data[..4] == b"Yaz0" {
        Yaz0Archive::new(Cursor::new(data))?.decompress()?
    } else {
        data
    };
    let rarc = Rarc::new(arc.as_slice()).expect("Rarc decompression error!");
    Ok(rarc.files().map(|(p, d)| (p, d.to_vec())).collect())
}
