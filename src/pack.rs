use cube_rs::{bmg::Bmg, rarc::Rarc, szs::yaz0_compress, virtual_fs::VirtualFile, Encode};
use log::info;
use std::{
    error::Error,
    fs::{remove_dir_all, remove_file, write},
    path::{Path, PathBuf},
};

use crate::commands::PackOptions;

pub fn try_pack(file: PathBuf, out: Option<&Path>, options: &PackOptions) -> Result<(), Box<dyn Error>> {
    let out_format = out.map(|p| {
        p.extension()
            .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
            .unwrap_or(String::from(""))
    });

    if file.is_dir() {
        for subfile in file.read_dir()? {
            try_pack(subfile?.path(), None, &options)?;
        }
    }

    let vfile = pack(&file, out_format.as_deref(), &options)?;
    if let Some(vfile) = vfile {
        info!("Packing {:?} => {:?}", &file, &vfile.path);
        write(out.unwrap_or(&vfile.path), &vfile.bytes)?;

        if options.delete_originals {
            if file.is_dir() {
                remove_dir_all(&file)?;
            } else {
                remove_file(&file)?;
            }
        }
    }

    Ok(())
}

fn pack(path: &Path, format: Option<&str>, options: &PackOptions) -> Result<Option<VirtualFile>, Box<dyn Error>> {
    let dest_format = format.or(guess_dest_format(path));
    match dest_format {
        Some("szs") | Some("arc") => {
            let mut rarc = Rarc::encode(path)?;

            if options.arc_yaz0_compress && dest_format.is_some_and(|f| f == "szs") {
                rarc = VirtualFile {
                    bytes: yaz0_compress(&rarc.bytes)?,
                    path: rarc.path.with_extension("szs"),
                };
            }

            if let Some(ext) = options.arc_extension.as_ref() {
                rarc.set_path(rarc.path.with_extension(ext));
            }

            Ok(Some(rarc))
        }
        Some("bmg") => {
            let vfile = VirtualFile::read(path)?;
            let bmg: Bmg = serde_json::from_slice(&vfile.bytes)?;
            Ok(Some(VirtualFile {
                path: path.with_extension("").with_extension("bmg"),
                bytes: bmg.write(),
            }))
        }
        _ => Ok(None),
    }
}

fn guess_dest_format(path: &Path) -> Option<&'static str> {
    let path_str = path.to_string_lossy();
    if path.is_dir() {
        // Never guess ARC, otherwise every nested folder will be ARC encoded
        return None;
    } else {
        if path_str.ends_with("json") {
            return Some("bmg");
        } else if path_str.ends_with("png") {
            return Some("bti");
        }
    }

    None
}
