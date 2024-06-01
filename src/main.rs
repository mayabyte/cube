mod commands;

use std::{
    error::Error,
    fs::{create_dir_all, read, write},
    io::{BufWriter, Cursor},
    path::{Path, PathBuf},
};

use clap::Parser;
use commands::{Cli, Commands};
use cube::{bti::BtiImage, szs::extract_szs};
use image::{ImageFormat, RgbaImage};

pub fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    match args.subcommand {
        Commands::Extract { files } => {
            for file in files {
                try_extract(&file)?;
            }
        }
        Commands::Pack { file: _ } => todo!(),
    }

    Ok(())
}

fn try_extract(file_path: &Path) -> Result<(), Box<dyn Error>> {
    let file_bytes = read(file_path)?;
    let extracted_files = extract(file_path, &file_bytes)?;
    for (path, bytes) in extracted_files {
        create_dir_all(path.parent().expect("File has no parent!"))?;
        write(&path, bytes)?;
    }
    Ok(())
}

fn extract(
    file_path: &Path,
    file_bytes: &Vec<u8>,
) -> Result<Vec<(PathBuf, Vec<u8>)>, Box<dyn Error>> {
    let path_string = file_path.to_string_lossy();
    let extension = path_string
        .rsplit_once('.')
        .map(|(_prefix, extension)| extension.to_ascii_lowercase());

    match extension.as_deref() {
        Some("szs") => {
            let extracted_folder_path = file_path.with_extension("");
            let contents = extract_szs(file_bytes.clone())?;

            let mut extracted = Vec::new();
            for (subpath, subfile_bytes) in contents {
                let subpath = extracted_folder_path.join(&subpath);
                match extract(&subpath, &subfile_bytes) {
                    Ok(subfiles) => extracted.extend(subfiles),
                    Err(e) => eprintln!("Couldn't extract {}: {e}", subpath.to_string_lossy()),
                }
            }

            Ok(extracted)
        }
        Some("bti") => {
            let bti = BtiImage::decode(&file_bytes);
            let mut dest = BufWriter::new(Cursor::new(Vec::new()));
            RgbaImage::from_vec(
                bti.width,
                bti.height,
                bti.pixels().flatten().cloned().collect(),
            )
            .unwrap()
            .write_to(&mut dest, ImageFormat::Png)?;
            Ok(vec![(
                file_path.with_extension("png"),
                dest.into_inner()?.into_inner(),
            )])
        }
        _ => Ok(vec![(file_path.to_owned(), file_bytes.to_owned())]),
    }
}
