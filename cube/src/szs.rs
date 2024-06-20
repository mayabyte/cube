use crate::rarc::Rarc;
use std::{io::Cursor, path::PathBuf};
use yaz0::{Error as Yaz0Error, Yaz0Archive, Yaz0Writer};

/// Extracts an (optionally Yaz0 compressed) SZS archive into a list of files with
/// their respective paths and raw contents.
pub fn extract_szs(data: Vec<u8>) -> Result<Vec<(PathBuf, Vec<u8>)>, Yaz0Error> {
    let arc = if &data[..4] == b"Yaz0" {
        Yaz0Archive::new(Cursor::new(data))?.decompress()?
    } else {
        data
    };
    std::fs::write("out.arc", &arc).unwrap();
    let rarc = Rarc::parse(arc.as_slice()).expect("Rarc decompression error!");
    Ok(rarc
        .files()
        .map(|(path, bytes)| (path, bytes.to_vec()))
        .collect())
}

pub fn yaz0_compress(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let yaz0_writer = Yaz0Writer::new(&mut out);
    yaz0_writer
        .compress_and_write(bytes, yaz0::CompressionLevel::Lookahead { quality: 10 })
        .expect("Yaz0 compression failed");
    out
}
