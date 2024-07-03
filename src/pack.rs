use std::{
    error::Error,
    fs::write,
    path::{Path, PathBuf},
};

use cube_rs::{bmg::Bmg, rarc::Rarc, virtual_fs::VirtualFile, Encode};

pub fn try_pack(file: PathBuf, out: Option<&Path>) -> Result<(), Box<dyn Error>> {
    let out_format = out
        .map(|p| p.extension())
        .flatten()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase());
    let vfile = pack(&file, out_format.as_deref())?;
    write(out.unwrap_or(&vfile.path), &vfile.bytes)?;
    Ok(())
}

/// Either returns the packed file if a packing scheme can be determined,
/// or the original file contents unmodified if no packing scheme could be determined
fn pack(path: &Path, format: Option<&str>) -> Result<VirtualFile, Box<dyn Error>> {
    let dest_format = format.or(guess_dest_format(path));
    println!("{:?}", dest_format);
    match dest_format {
        Some("szs") => Ok(Rarc::encode(path)?),
        Some("bmg") => {
            let vfile = VirtualFile::read(path)?;
            let bmg: Bmg = serde_json::from_slice(&vfile.bytes)?;
            Ok(VirtualFile {
                path: path.with_extension(""), // Removes the last component of the extension ("json" in this case)
                bytes: bmg.write(),
            })
        }
        _ => Ok(VirtualFile::read(path)?),
    }
}

fn guess_dest_format(path: &Path) -> Option<&'static str> {
    let path_str = path.to_string_lossy();
    if path.is_dir() {
        if path_str.ends_with(".szs") {
            return Some("szs");
        }
    } else {
        if path_str.ends_with("bmg.json") {
            return Some("bmg");
        } else if path_str.ends_with("bti.png") {
            return Some("bti");
        }
    }

    None
}
