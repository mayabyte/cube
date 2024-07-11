use std::{
    cmp::min,
    collections::VecDeque,
    fmt::Display,
    fs::{metadata, read, read_dir},
    path::{Path, PathBuf},
};

use itertools::Itertools;

use crate::{
    util::{read_str_until_null, read_u16, read_u32},
    virtual_fs::VirtualFile,
    Decode, Encode,
};

pub struct Rarc<'a> {
    data: &'a [u8],
    pub header: RarcHeader,
    pub info_block: RarcInfoBlock,
    pub nodes: Vec<RarcNode>,
    pub files: Vec<RarcFile>,
}

impl<'a> Decode for Rarc<'a> {
    type Out = Vec<VirtualFile>;
    fn decode(&self) -> Self::Out {
        self.files()
            .map(|(path, bytes)| VirtualFile {
                path,
                bytes: bytes.to_vec(),
            })
            .collect()
    }
}

impl<'a> Encode for Rarc<'a> {
    type Error = RarcError;
    fn encode<P: AsRef<Path>>(root: P) -> Result<VirtualFile, Self::Error> {
        let root = root.as_ref();
        if !metadata(root)?.is_dir() {
            return Err(RarcError::NotADirError);
        }

        let mut nodes = vec![RarcNode {
            node_name: String::from("ROOT"),
            name_offset: 5, // String table always starts with "." and ".." plus their null terminators, then the root node name
            num_files: 0,
            first_file_index: 0,
        }];
        let mut file_entries = vec![];
        let mut non_dir_file_entries = 0;
        let mut string_table = vec![];
        let mut file_data = vec![];

        // Initialize the string table
        string_table.extend(b".\0");
        string_table.extend(b"..\0");
        string_table.extend(root.file_name().unwrap().to_string_lossy().as_bytes());
        string_table.push(b'\0');

        let mut dir_queue = VecDeque::new();
        dir_queue.push_back(root.to_owned());

        while !dir_queue.is_empty() {
            let dir = dir_queue.pop_front().unwrap();
            let mut num_files = 2; // for . and .. added at the end

            for dir_entry in read_dir(&dir)?
                .filter_map(|entry| entry.ok())
                .sorted_by_key(|entry| entry.file_name())
            {
                if dir_entry.file_type()?.is_dir() {
                    dir_queue.push_back(dir_entry.path());
                    let file_name = dir_entry.file_name().to_string_lossy().into_owned();
                    file_entries.push(RarcFile {
                        name: file_name.clone(),
                        index: 0xFFFF,
                        name_offset: string_table.len() as u16,
                        data_size: 16, // always 16 for folders besides ROOT
                        data_offset_or_node_index: nodes.len() as u32,
                        file_type_flags: 0x0200, // Always this value for folders
                    });
                    num_files += 1;

                    nodes.push(RarcNode {
                        node_name: file_name[..4].to_ascii_uppercase(),
                        name_offset: string_table.len() as u32,
                        num_files: 0,        // Will be updated later
                        first_file_index: 0, // Will be updated later
                    });

                    string_table.extend(file_name.as_bytes());
                    string_table.push(b'\0');
                } else {
                    let data = read(dir_entry.path())?;
                    let file_name = dir_entry.file_name().to_string_lossy().into_owned();
                    file_entries.push(RarcFile {
                        name: file_name.clone(),
                        index: non_dir_file_entries,
                        name_offset: string_table.len() as u16,
                        data_size: data.len() as u32,
                        data_offset_or_node_index: file_data.len() as u32,
                        file_type_flags: 0x1100,
                    });
                    non_dir_file_entries += 1;
                    string_table.extend(file_name.bytes());
                    string_table.push(b'\0');
                    file_data.extend(data);
                    num_files += 1;
                }
            }

            let node_name = to_node_name(&dir, &root).unwrap();
            let node_idx = nodes
                .iter()
                .find_position(|node| &node.node_name == &node_name)
                .expect(&format!(
                    "Expected to find a RarcNode named \"{node_name}\" while packing!"
                ))
                .0;

            // All directories contain . and .. files in the output archive
            file_entries.push(RarcFile {
                name: ".".to_owned(),
                index: file_entries.len() as u16,
                name_offset: 0,
                data_size: 16,
                data_offset_or_node_index: node_idx as u32,
                file_type_flags: 0x0200,
            });
            let parent_node_idx = dir
                .parent()
                .map(|parent| {
                    let parent_node_name = to_node_name(&parent, &root)?;
                    nodes.iter().find_position(|node| &node.node_name == &parent_node_name)
                })
                .flatten()
                .map(|(idx, _)| idx as u32)
                .unwrap_or(u32::MAX);
            file_entries.push(RarcFile {
                name: "..".to_owned(),
                index: file_entries.len() as u16,
                name_offset: 2,
                data_size: 16,
                data_offset_or_node_index: parent_node_idx,
                file_type_flags: 0x0200, // Always this value for folders
            });

            // Update this Node's number of files and first file index
            let node = &mut nodes[node_idx];
            node.num_files = num_files as u16;
            node.first_file_index = file_entries.len() as u32 - node.num_files as u32;
        }

        // Pad end of string table
        while string_table.len() % 32 != 0 {
            string_table.push(0);
        }

        // Construct the final header and info block
        let node_list_offset = 0x20; // relative to start of info block
        let file_entries_list_offset = node_list_offset + (nodes.len() * 0x10) as u32;
        let string_table_offset = file_entries_list_offset + (file_entries.len() * 0x14) as u32;
        let file_data_list_offset = string_table_offset + string_table.len() as u32;
        let final_file_length = file_data_list_offset + file_data.len() as u32 + 0x20;
        let header = RarcHeader {
            file_data_length: file_data.len() as u32,
            file_length: final_file_length,
            file_data_list_offset,
        };
        let info_block = RarcInfoBlock {
            num_nodes: nodes.len() as u32,
            num_file_entries: file_entries.len() as u32,
            string_table_length: string_table.len() as u32,
            num_files: file_entries.iter().filter(|entry| !entry.is_dir()).count() as u16,
            node_list_offset,
            file_entries_list_offset,
            string_table_offset,
        };

        // Final RARC file is structured as follows:
        // header: 0x20
        // info block: 0x20
        // node list: num_nodes x 0x10
        // file entry list: num_file_entries x 0x14
        // string table
        // file data

        let mut final_file_data = Vec::with_capacity(final_file_length as usize);
        final_file_data.extend(header.write());
        final_file_data.extend(info_block.write());
        for node in nodes {
            final_file_data.extend(node.write(&string_table));
        }
        for file_entry in file_entries {
            final_file_data.extend(file_entry.write());
        }
        final_file_data.extend(string_table);
        final_file_data.extend(file_data);

        Ok(VirtualFile {
            path: root.with_extension("arc"),
            bytes: final_file_data,
        })
    }
}

impl<'a> Rarc<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Rarc<'a>, RarcError> {
        if &data[0..4] != b"RARC" {
            return Err(RarcError::MagicError(0));
        }

        let file_length = read_u32(data, 0x4);
        if file_length != data.len() as u32 {
            return Err(RarcError::MetadataError(file_length));
        }

        let header_length = read_u32(data, 0x8);
        if header_length != 0x20 {
            return Err(RarcError::MagicError(1));
        }

        let file_data_list_offset = read_u32(data, 0xC) + header_length;
        let unk1 = read_u32(data, 0x1C);
        if unk1 != 0 {
            return Err(RarcError::MagicError(2));
        }

        let file_data_length = read_u32(data, 0x10);

        let num_nodes = read_u32(data, header_length);
        let node_list_offset = read_u32(data, header_length + 0x4) + header_length;
        let num_file_entries = read_u32(data, header_length + 0x8);
        let file_entries_list_offset = read_u32(data, header_length + 0x0C) + header_length;
        let string_table_length = read_u32(data, header_length + 0x10);
        let string_table_offset = read_u32(data, header_length + 0x14) + header_length;
        let num_files = read_u16(data, header_length + 0x18);

        let mut nodes = Vec::with_capacity(num_nodes as usize);
        for node_idx in 0..num_nodes {
            nodes.push(RarcNode::read(data, node_list_offset + node_idx * 0x10));
        }

        let mut files = Vec::with_capacity(num_file_entries as usize);
        for file_idx in 0..num_file_entries {
            files.push(RarcFile::read(
                data,
                file_entries_list_offset + file_idx * 0x14,
                string_table_offset,
            ));
        }

        Ok(Rarc {
            data,
            header: RarcHeader {
                file_length,
                file_data_list_offset,
                file_data_length,
            },
            info_block: RarcInfoBlock {
                num_nodes,
                node_list_offset,
                num_file_entries,
                file_entries_list_offset,
                string_table_length,
                string_table_offset,
                num_files,
            },
            nodes,
            files,
        })
    }

    pub fn files(&self) -> impl Iterator<Item = (PathBuf, &[u8])> {
        let root_node = &self.nodes[0];
        let files_with_paths = self.files_for_node(root_node, PathBuf::new());
        files_with_paths
            .into_iter()
            .filter(|(_, file)| ![".", ".."].contains(&&file.name[..]))
            .map(|(mut path, file)| {
                path.push(&file.name[..]);
                let file_start = (self.header.file_data_list_offset + file.data_offset_or_node_index) as usize;
                let file_end = file_start + file.data_size as usize;
                (path, &self.data[file_start..file_end])
            })
    }

    fn files_for_node(&self, node: &RarcNode, parent_path: PathBuf) -> Vec<(PathBuf, &RarcFile)> {
        let file_entries =
            &self.files[node.first_file_index as usize..(node.first_file_index + node.num_files as u32) as usize];
        let (dirs, files): (Vec<_>, Vec<_>) = file_entries.iter().partition(|e| e.is_dir());
        let mut files_with_paths: Vec<_> = files.into_iter().map(|f| (parent_path.clone(), f)).collect();
        for file in dirs {
            if ![".", ".."].contains(&&file.name[..]) {
                let sub_node = &self.nodes[file.data_offset_or_node_index as usize];
                let mut new_parent_path = parent_path.clone();
                new_parent_path.push(&file.name[..]);
                files_with_paths.extend(self.files_for_node(sub_node, new_parent_path));
            }
        }
        files_with_paths
    }
}

#[derive(Debug)]
pub struct RarcHeader {
    pub file_length: u32,
    pub file_data_list_offset: u32,
    pub file_data_length: u32,
}

impl RarcHeader {
    pub fn write(&self) -> [u8; 0x20] {
        let mut out = [0u8; 0x20];
        out[..4].copy_from_slice(b"RARC");
        out[4..8].copy_from_slice(&self.file_length.to_be_bytes());
        out[8..0xC].copy_from_slice(&0x20u32.to_be_bytes());
        out[0xC..0x10].copy_from_slice(&self.file_data_list_offset.to_be_bytes());
        out[0x10..0x14].copy_from_slice(&self.file_data_length.to_be_bytes());
        out[0x14..0x18].copy_from_slice(&self.file_data_length.to_be_bytes()); // Intentional duplication
        out
    }
}

#[derive(Debug)]
pub struct RarcInfoBlock {
    pub num_nodes: u32,
    pub node_list_offset: u32, // relative to START of this block, so add 0x20 for absolute offset
    pub num_file_entries: u32,
    pub file_entries_list_offset: u32, // relative to START of this block
    pub string_table_length: u32,
    pub string_table_offset: u32,
    pub num_files: u16, // the number of RarcFiles that represent actual files, not directories
}

impl RarcInfoBlock {
    pub fn write(&self) -> [u8; 0x20] {
        let mut out = [0u8; 0x20];
        out[..4].copy_from_slice(&self.num_nodes.to_be_bytes());
        out[4..8].copy_from_slice(&self.node_list_offset.to_be_bytes());
        out[8..0xC].copy_from_slice(&self.num_file_entries.to_be_bytes());
        out[0xC..0x10].copy_from_slice(&self.file_entries_list_offset.to_be_bytes());
        out[0x10..0x14].copy_from_slice(&self.string_table_length.to_be_bytes());
        out[0x14..0x18].copy_from_slice(&self.string_table_offset.to_be_bytes());
        out[0x18..0x1A].copy_from_slice(&self.num_files.to_be_bytes());
        out
    }
}

#[derive(Debug)]
pub struct RarcNode {
    pub node_name: String, // 4 character uppercase ID
    pub name_offset: u32,  // this is the actual folder name
    pub num_files: u16,    // number of ALL file entries, not just those that aren't directories
    pub first_file_index: u32,
}

impl RarcNode {
    fn read(data: &[u8], node_offset: u32) -> Self {
        let node_name = std::str::from_utf8(&read_u32(data, node_offset).to_be_bytes())
            .expect("Invalid UTF8 in RARC node name")
            .to_owned();
        let name_offset = read_u32(data, node_offset + 0x4);
        let num_files = read_u16(data, node_offset + 0xA);
        let first_file_index = read_u32(data, node_offset + 0xC);

        RarcNode {
            node_name,
            name_offset,
            num_files,
            first_file_index,
        }
    }

    fn write(&self, string_table: &[u8]) -> [u8; 0x10] {
        let mut out = [0u8; 0x10];
        out[..4].copy_from_slice(self.node_name.as_bytes());
        out[4..8].copy_from_slice(&self.name_offset.to_be_bytes());
        let full_name = read_str_until_null(string_table, self.name_offset);
        out[8..0xA].copy_from_slice(&string_hash(&full_name).to_be_bytes());
        out[0xA..0xC].copy_from_slice(&self.num_files.to_be_bytes());
        out[0xC..].copy_from_slice(&self.first_file_index.to_be_bytes());
        out
    }
}

#[derive(Debug)]
pub struct RarcFile {
    pub name: String,
    pub index: u16,
    pub name_offset: u16,
    pub data_size: u32,
    pub data_offset_or_node_index: u32,
    pub file_type_flags: u16,
}

impl RarcFile {
    fn read(data: &[u8], file_offset: u32, string_list_offset: u32) -> Self {
        let index = read_u16(data, file_offset);
        let type_and_name_offset = read_u32(data, file_offset + 0x4);
        let data_offset_or_node_index = read_u32(data, file_offset + 0x8);
        let data_size = read_u32(data, file_offset + 0xC);
        let file_type_flags = (type_and_name_offset & 0xFF000000) >> 24;
        let name_offset = type_and_name_offset & 0x00FFFFFF;
        let name = read_str_until_null(data, string_list_offset + name_offset).into_owned();

        RarcFile {
            name,
            index,
            name_offset: name_offset as u16,
            data_size,
            data_offset_or_node_index,
            file_type_flags: file_type_flags as u16,
        }
    }

    fn write(&self) -> [u8; 0x14] {
        let mut out = [0u8; 0x14];
        out[..2].copy_from_slice(&self.index.to_be_bytes());
        out[2..4].copy_from_slice(&string_hash(&self.name).to_be_bytes());
        out[4..6].copy_from_slice(&self.file_type_flags.to_be_bytes());
        out[6..8].copy_from_slice(&self.name_offset.to_be_bytes());
        out[8..0xC].copy_from_slice(&self.data_offset_or_node_index.to_be_bytes());
        out[0xC..0x10].copy_from_slice(&self.data_size.to_be_bytes());
        // rest is unused / always 0
        out
    }
    fn is_dir(&self) -> bool {
        self.file_type_flags & 0x02 != 0
    }
}

fn to_node_name(p: &Path, root: &Path) -> Option<String> {
    if p == root {
        Some("ROOT".to_string())
    } else {
        let file_name = p.file_name()?.to_string_lossy().to_ascii_uppercase();
        let mut node_name = file_name[..min(4, file_name.len())].to_owned();
        while node_name.len() < 4 {
            node_name.push('\0');
        }
        Some(node_name)
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum RarcError {
    MagicError(usize),
    MetadataError(u32),
    NotADirError,
    IOError(std::io::Error),
}

impl Display for RarcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RarcError::MagicError(magic) => write!(f, "Error in magic numbers: {magic}"),
            RarcError::MetadataError(metadata) => write!(f, "Inconsistent metadata: {metadata}"),
            RarcError::NotADirError => write!(f, "Can only compress directories"),
            RarcError::IOError(e) => write!(f, "IO Error while processing RARC file: {e}"),
        }
    }
}

impl std::error::Error for RarcError {}

impl From<std::io::Error> for RarcError {
    fn from(value: std::io::Error) -> Self {
        RarcError::IOError(value)
    }
}

fn string_hash(string: &str) -> u16 {
    let mut hash = 0u16;
    for c in string.bytes() {
        hash = hash.wrapping_mul(3);
        hash = hash.wrapping_add(c as u16);
    }
    hash
}
