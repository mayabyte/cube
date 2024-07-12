use crate::commands::ExtractOptions;
use cube_rs::{bmg::Bmg, bti::BtiImage, iso::extract_iso, szs::extract_szs, virtual_fs::VirtualFile};
use image::{ImageFormat, RgbaImage};
use log::{debug, error, info};
use std::{
    error::Error,
    fs::{create_dir_all, write},
    io::{BufWriter, Cursor},
    path::{Path, PathBuf},
};

pub fn try_extract(files: Vec<PathBuf>, out: Option<&Path>, options: ExtractOptions) -> Result<(), Box<dyn Error>> {
    for path in files {
        extract_and_write(&path, out, options)?;
    }

    Ok(())
}

fn extract_and_write(path: &Path, out_path: Option<&Path>, options: ExtractOptions) -> Result<(), Box<dyn Error>> {
    let vfile = VirtualFile::read(path)?;
    let extracted_files = extract(vfile, options)?;

    if extracted_files.len() < 1 {
        return Err("No output files?".into());
    }

    // If we have exactly one extracted file, the output path becomes its filename
    if extracted_files.len() == 1 {
        let out_file = &extracted_files[0];
        let out_path = out_path.unwrap_or(&out_file.path);
        create_dir_all(out_path.parent().expect("Path has no parent"))?;
        write(out_path, &out_file.bytes)?;
    }
    // We have multiple extracted files.
    else {
        // If the user provided an output path, that becomes the name of the folder
        // we put them in.
        let mut parent = out_path.map(ToOwned::to_owned);

        // If the user did not provide an output path we use the name of the input
        // file minus its file extension as the output folder name
        if parent.is_none() {
            let out_path = path.with_extension("");
            // ... unless all the extracted files already start with this path
            let should_create_folder = !extracted_files.iter().all(|ef| ef.path.starts_with(&out_path));
            if should_create_folder {
                parent = Some(out_path);
            }
        }
        // If the user provided multiple input files and there are multiple output
        // files, we just dump everything in the current directory (do nothing).

        for mut extracted in extracted_files {
            if let Some(out_path) = &parent {
                extracted.set_path(out_path.join(&extracted.path.strip_prefix(path).unwrap_or(&extracted.path)));
            }
            debug!("Writing file {:?}", &extracted.path);
            create_dir_all(&extracted.path.parent().expect("Path has no parent"))?;
            write(extracted.path, &extracted.bytes)?;
        }
    }

    Ok(())
}

fn extract(vfile: VirtualFile, options: ExtractOptions) -> Result<Vec<VirtualFile>, Box<dyn Error>> {
    let path_string = vfile.path.to_string_lossy();
    let extension = path_string
        .rsplit_once('.')
        .map(|(_prefix, extension)| extension.to_ascii_lowercase());

    match extension.as_deref() {
        Some("iso") => {
            let extracted: Vec<VirtualFile> = extract_iso(&vfile.path)?
                .into_iter()
                .flat_map(|vfile| extract(vfile, options))
                .flatten()
                .collect();
            info!("Extracted {path_string} into {} files", extracted.len());
            Ok(extracted)
        }
        Some("szs") | Some("arc") => {
            let mut extracted_folder_path = vfile.path.clone();
            if !options.szs_preserve_extension {
                extracted_folder_path.set_extension("");
            }
            let contents = extract_szs(vfile.bytes.clone())?;

            let mut extracted = Vec::new();
            for subfile in contents {
                let subpath = extracted_folder_path.join(&subfile.path);
                match extract(subfile.with_path(subpath.clone()), options) {
                    Ok(subfiles) => extracted.extend(subfiles),
                    Err(e) => error!("Couldn't extract {}: {e}", subpath.to_string_lossy()),
                }
            }

            info!("Extracted {path_string} into {} files", extracted.len());
            Ok(extracted)
        }
        Some("bti") if options.extract_bti => {
            let bti = BtiImage::decode(&vfile.bytes);
            let mut dest = BufWriter::new(Cursor::new(Vec::new()));
            RgbaImage::from_vec(bti.width, bti.height, bti.pixels().flatten().cloned().collect())
                .unwrap()
                .write_to(&mut dest, ImageFormat::Png)?;

            let output_path = vfile.path.with_extension("bti.png");
            info!("Extracted {path_string} => {output_path:?}");
            Ok(vec![VirtualFile {
                path: output_path,
                bytes: dest.into_inner()?.into_inner(),
            }])
        }
        Some("bmg") if options.extract_bmg => {
            let bmg = Bmg::read(&vfile.bytes)?;
            let output_path = vfile.path.with_extension("bmg.json");
            info!("Extracted {path_string} => {output_path:?}");
            Ok(vec![VirtualFile {
                path: output_path,
                bytes: serde_json::to_vec_pretty(&bmg)?,
            }])
        }
        _ => Ok(vec![vfile]),
    }
}
