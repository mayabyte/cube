use cube_rs::{bmg::Bmg, rarc::Rarc, virtual_fs::VirtualFile, Encode};
use log::info;
use std::{
    error::Error,
    fs::{remove_dir_all, remove_file, write},
    path::{Path, PathBuf},
};

use crate::commands::PackOptions;

pub fn try_pack(file: PathBuf, out: Option<&Path>, options: PackOptions) -> Result<(), Box<dyn Error>> {
    if file.is_dir() {
        for subfile in file.read_dir()? {
            try_pack(subfile?.path(), None, options)?;
        }
    }

    let out_format = out.map(|p| {
        p.extension()
            .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
            .unwrap_or(String::from(""))
    });
    let vfile = pack(&file, out_format.as_deref())?;
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

fn pack(path: &Path, format: Option<&str>) -> Result<Option<VirtualFile>, Box<dyn Error>> {
    let dest_format = format.or(guess_dest_format(path));
    match dest_format {
        Some("szs") => Ok(Some(Rarc::encode(path)?)),
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
        return Some("szs");
    } else {
        if path_str.ends_with("json") {
            return Some("bmg");
        } else if path_str.ends_with("png") {
            return Some("bti");
        }
    }

    None
}
