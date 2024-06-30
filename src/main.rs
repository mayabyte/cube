mod commands;

use std::{
    error::Error,
    fs::{self, create_dir_all, read, write},
    io::{BufWriter, Cursor},
    path::Path,
};

use clap::Parser;
use commands::{Cli, Commands};
use cube_rs::{
    bti::BtiImage, iso::extract_iso, rarc::Rarc, szs::extract_szs, virtual_fs::VirtualFile, Encode,
};
use image::{ImageFormat, RgbaImage};

pub fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    match args.subcommand {
        Commands::Extract { files, bti } => {
            for file in files {
                try_extract(&file, bti)?;
            }
        }
        Commands::Pack { file, out } => {
            if file.is_dir() {
                let packed = Rarc::encode(&file)?;
                fs::write(out.unwrap_or(packed.path), packed.bytes)?;
            } else {
                eprintln!("Only folders supported for RARC packing currently");
            }
        }
    }

    Ok(())
}

fn try_extract(file_path: &Path, extract_bti: bool) -> Result<(), Box<dyn Error>> {
    let file_bytes = read(file_path)?;
    let extracted_files = extract(
        VirtualFile {
            path: file_path.to_owned(),
            bytes: file_bytes,
        },
        extract_bti,
    )?;
    for vfile in extracted_files {
        create_dir_all(vfile.path.parent().expect("File has no parent!"))?;
        write(&vfile.path, vfile.bytes)?;
    }
    Ok(())
}

fn extract(vfile: VirtualFile, extract_bti: bool) -> Result<Vec<VirtualFile>, Box<dyn Error>> {
    let path_string = vfile.path.to_string_lossy();
    let extension = path_string
        .rsplit_once('.')
        .map(|(_prefix, extension)| extension.to_ascii_lowercase());

    match extension.as_deref() {
        Some("iso") => {
            let extracted_folder_path = vfile.path.with_extension("");
            Ok(extract_iso(vfile.path)?
                .into_iter()
                .flat_map(|vfile| extract(vfile, extract_bti))
                .flatten()
                .map(|mut f| {
                    f.set_path(extracted_folder_path.join(&f.path));
                    f
                })
                .collect())
        }
        Some("szs") => {
            let extracted_folder_path = vfile.path.with_extension("");
            let contents = extract_szs(vfile.bytes.clone())?;

            let mut extracted = Vec::new();
            for subfile in contents {
                let subpath = extracted_folder_path.join(&subfile.path);
                match extract(subfile.with_path(subpath.clone()), extract_bti) {
                    Ok(subfiles) => extracted.extend(subfiles),
                    Err(e) => eprintln!("Couldn't extract {}: {e}", subpath.to_string_lossy()),
                }
            }

            Ok(extracted)
        }
        Some("bti") if extract_bti => {
            println!("stop");
            let bti = BtiImage::decode(&vfile.bytes);
            let mut dest = BufWriter::new(Cursor::new(Vec::new()));
            RgbaImage::from_vec(
                bti.width,
                bti.height,
                bti.pixels().flatten().cloned().collect(),
            )
            .unwrap()
            .write_to(&mut dest, ImageFormat::Png)?;
            Ok(vec![VirtualFile {
                path: vfile.path.with_extension("png"),
                bytes: dest.into_inner()?.into_inner(),
            }])
        }
        _ => Ok(vec![vfile.clone()]),
    }
}
