use crate::util::{from_hex_string, pad_to, read_u16, read_u32, read_u64, to_hex_string};
use encoding_rs::{SHIFT_JIS, UTF_16BE, UTF_8, WINDOWS_1252};
use log::debug;
use serde::{Deserialize, Serialize};
use std::cmp::max;
use thiserror::Error;

/// BMGs are indexed text archives used in GameCube, Wii, and some WiiU games
/// made by Nintendo.
/// Documentation on BMGs:
/// - Custom MKWii Wiki: https://wiki.tockdom.com/wiki/BMG_(File_Format)
/// - Pikmin Technical Knowledge Base: https://pikmintkb.com/wiki/BMG_file
/// - PikminBMGTool by RenolY2: https://github.com/RenolY2/pikminBMG/blob/master/pikminBMGtool.py
#[derive(Debug)]
pub struct Bmg {
    header: BmgHeader,
    text_index_table: TextIndexTable,         // INF1
    string_pool: StringPool,                  // DAT1
    message_id_table: Option<MessageIdTable>, // MID1
    unknown_sections: Vec<UnknownSection>,
}

impl Bmg {
    pub fn new(text_encoding: TextEncoding) -> Bmg {
        Bmg {
            header: BmgHeader::new(text_encoding),
            text_index_table: TextIndexTable::new(),
            string_pool: StringPool::new(),
            message_id_table: None,
            unknown_sections: Vec::with_capacity(0), // don't allocate for unknown sections
        }
    }

    pub fn read(data: &[u8]) -> Result<Bmg, BmgError> {
        let mut bmg = Bmg {
            header: BmgHeader::read(data)?,
            text_index_table: TextIndexTable::new(),
            string_pool: StringPool::new(),
            message_id_table: None,
            unknown_sections: Vec::with_capacity(0),
        };

        let mut section_start = BmgHeader::SIZE;
        for _ in 0..bmg.header.num_blocks {
            // align if necessary
            while bmg.is_block_aligned() && section_start % 32 != 0 {
                section_start += 1;
            }

            // read each section based on its magic value
            match &data[section_start..section_start + 4] {
                TextIndexTable::MAGIC => {
                    bmg.text_index_table = TextIndexTable::read(&data[section_start..])?;
                    section_start += bmg.text_index_table.section_size as usize;
                }
                StringPool::MAGIC => {
                    bmg.string_pool = StringPool::read(&data[section_start..])?;
                    section_start += bmg.string_pool.section_size as usize;
                }
                MessageIdTable::MAGIC => {
                    bmg.message_id_table = Some(MessageIdTable::read(&data[section_start..])?);
                    section_start += bmg.message_id_table.as_ref().unwrap().section_size as usize;
                }
                _ => {
                    bmg.unknown_sections.push(UnknownSection::read(&data[section_start..])?);
                    section_start += bmg.unknown_sections.last().unwrap().section_size as usize;
                }
            }
        }

        Ok(bmg)
    }

    pub fn write(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.header.file_size as usize);
        out.extend(self.header.write());
        out.extend(self.text_index_table.write());
        if self.is_block_aligned() {
            pad_to(&mut out, 32);
        }
        out.extend(self.string_pool.write());
        if let Some(message_id_table) = self.message_id_table.as_ref() {
            if self.is_block_aligned() {
                pad_to(&mut out, 32);
            }
            out.extend(message_id_table.write());
        }
        for unk_section in self.unknown_sections.iter() {
            if self.is_block_aligned() {
                pad_to(&mut out, 32);
            }
            out.extend(unk_section.write());
        }

        out
    }

    fn is_block_aligned(&self) -> bool {
        self.header.encoding == TextEncoding::Undefined
    }

    fn message_id_table_mut(&mut self) -> &mut MessageIdTable {
        if self.message_id_table.is_none() {
            self.message_id_table = Some(MessageIdTable::new());
            self.header.num_blocks += 1;
        }
        self.message_id_table.as_mut().unwrap()
    }

    pub fn messages(&self) -> impl Iterator<Item = BmgMessage> + '_ {
        self.text_index_table
            .messages
            .iter()
            .enumerate()
            .map(|(idx, index_entry)| {
                let attributes = to_hex_string(&index_entry.attributes);
                let message = self
                    .header
                    .encoding
                    .decode(&self.string_pool.strings[index_entry.text_offset as usize..]);
                let index = self.message_id_table.as_ref().map(|mids| mids.message_ids[idx]);
                BmgMessage {
                    message,
                    index,
                    attributes,
                }
            })
    }

    pub fn set_file_id(&mut self, id: u16) {
        self.text_index_table.bmg_file_id = id;
    }

    pub fn set_default_color(&mut self, color: u8) {
        self.text_index_table.default_color = color;
    }

    pub fn set_message_id_format(&mut self, format: u8) {
        self.message_id_table_mut().format = format;
    }

    pub fn set_message_id_info(&mut self, info: u8) {
        self.message_id_table_mut().info = info;
    }

    pub fn add_message(&mut self, message: BmgMessage) {
        let encoded_message = self.header.encoding.encode(&message.message);
        self.text_index_table.add_message(
            self.string_pool.strings.len() as u32,
            from_hex_string(&message.attributes).expect("Invalid hex string for message attributes"),
        );
        self.string_pool.add_message(&encoded_message);
        if let Some(message_id) = message.index {
            self.message_id_table_mut().add_message(message_id);
        }
        self.header.file_size = BmgHeader::SIZE as u32
            + self.text_index_table.section_size
            + self.string_pool.section_size
            + self.message_id_table.as_ref().map(|t| t.section_size).unwrap_or(0)
            + self.unknown_sections.iter().map(|s| s.section_size).sum::<u32>();
    }
}

impl From<BmgSerialize> for Bmg {
    fn from(ser: BmgSerialize) -> Self {
        let mut bmg = Bmg::new(ser.metadata.encoding);
        bmg.set_file_id(ser.metadata.bmg_file_id);
        bmg.set_default_color(ser.metadata.default_color);
        if let Some(format) = ser.metadata.message_id_format {
            bmg.set_message_id_format(format)
        };
        if let Some(info) = ser.metadata.message_id_info {
            bmg.set_message_id_info(info);
        }
        for message in ser.messages {
            bmg.add_message(message);
        }
        bmg
    }
}

impl Serialize for Bmg {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        BmgSerialize {
            messages: self.messages().collect(),
            metadata: BmgSerializeMetadata {
                encoding: self.header.encoding,
                bmg_file_id: self.text_index_table.bmg_file_id,
                default_color: self.text_index_table.default_color,
                message_id_format: self.message_id_table.as_ref().map(|t| t.format),
                message_id_info: self.message_id_table.as_ref().map(|t| t.info),
            },
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Bmg {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        BmgSerialize::deserialize(deserializer).map(Into::into)
    }
}

/// A container for the various aspects of a string stored in a BMG file. This does not
/// map onto any part of the file format, and is just a convenience for working with messages.
#[derive(Debug, Serialize, Deserialize)]
pub struct BmgMessage {
    pub message: String,
    pub index: Option<MessageId>,
    pub attributes: String,
}

/// The minimum set of metadata needed to perfectly reconstruct the BMG from a serialized format,
/// such as JSON. Serializing the raw BMG file format structs is not very human friendly.
#[derive(Debug, Serialize, Deserialize)]
struct BmgSerializeMetadata {
    encoding: TextEncoding,
    bmg_file_id: u16,
    default_color: u8,
    message_id_format: Option<u8>,
    message_id_info: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BmgSerialize {
    metadata: BmgSerializeMetadata,
    messages: Vec<BmgMessage>,
}

#[derive(Debug)]
struct BmgHeader {
    file_size: u32, // bytes
    /// Number of sections
    num_blocks: u32,
    encoding: TextEncoding,
    _unk0: u8,
    _unk1: u16,
    _unk2: u64,
    _unk3: u32,
}

impl BmgHeader {
    const MAGIC: &'static [u8] = b"MESGbmg1";
    const SIZE: usize = 0x20;

    pub fn new(text_encoding: TextEncoding) -> BmgHeader {
        BmgHeader {
            file_size: BmgHeader::SIZE as u32,
            num_blocks: 2,
            encoding: text_encoding,
            _unk0: 0,
            _unk1: 0,
            _unk2: 0,
            _unk3: 0,
        }
    }

    pub fn write(&self) -> [u8; BmgHeader::SIZE] {
        let mut out = [0u8; BmgHeader::SIZE];
        out[..0x8].copy_from_slice(BmgHeader::MAGIC); // magic
        if self.encoding == TextEncoding::Undefined {
            out[0x8..0xC].copy_from_slice(&self.num_blocks.to_be_bytes());
        } else {
            out[0x8..0xC].copy_from_slice(&self.file_size.to_be_bytes());
        }
        out[0xC..0x10].copy_from_slice(&self.num_blocks.to_be_bytes());
        out[0x10] = self.encoding.to_byte().to_be();
        out[0x11] = self._unk0.to_be();
        out[0x12..0x14].copy_from_slice(&self._unk1.to_be_bytes());
        out[0x14..0x1C].copy_from_slice(&self._unk2.to_be_bytes());
        out[0x1C..].copy_from_slice(&self._unk3.to_be_bytes());

        out
    }

    /// Assumes the first 0x20 bytes of the provided slice are a valid BMG header.
    pub fn read(data: &[u8]) -> Result<BmgHeader, BmgError> {
        if &data[..0x8] != BmgHeader::MAGIC {
            return Err(BmgError::InvalidHeaderMagic);
        }

        let file_size = read_u32(data, 0x8);
        let num_blocks = read_u32(data, 0xC);
        let encoding_byte = data[0x10];
        let encoding = TextEncoding::from_byte(encoding_byte).ok_or(BmgError::InvalidTextEncoding(encoding_byte))?;
        let _unk0 = data[0x11];
        let _unk1 = read_u16(data, 0x12);
        let _unk2 = read_u64(data, 0x14);
        let _unk3 = read_u32(data, 0x1C);

        let header = BmgHeader {
            file_size,
            num_blocks,
            encoding,
            _unk0,
            _unk1,
            _unk2,
            _unk3,
        };
        debug!("Read {header:?}",);

        Ok(header)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextEncoding {
    Undefined, // Usually CP1252. Value used by some older GameCube games.
    CP1252,
    UTF16,
    ShiftJIS,
    UTF8,
}

impl TextEncoding {
    pub fn from_byte(b: u8) -> Option<TextEncoding> {
        match b {
            0 => Some(TextEncoding::Undefined),
            1 => Some(TextEncoding::CP1252),
            2 => Some(TextEncoding::UTF16),
            3 => Some(TextEncoding::ShiftJIS),
            4 => Some(TextEncoding::UTF8),
            _ => None,
        }
    }

    pub fn to_byte(&self) -> u8 {
        match self {
            TextEncoding::Undefined => 0,
            TextEncoding::CP1252 => 1,
            TextEncoding::UTF16 => 2,
            TextEncoding::ShiftJIS => 3,
            TextEncoding::UTF8 => 4,
        }
    }

    fn codepoint_size(&self) -> usize {
        match self {
            TextEncoding::UTF16 => 2,
            _ => 1,
        }
    }

    /// Decodes raw null-terminated bytes into a string using this format
    pub fn decode<'a>(&self, data: &'a [u8]) -> String {
        fn read_codepoint(data: &[u8], offset: usize, codepoint_size: usize) -> u16 {
            if codepoint_size == 2 {
                read_u16(data, offset as u32)
            } else {
                u8::from_be(data[offset]) as u16
            }
        }

        let codepoint_size = self.codepoint_size();
        let mut blocks = Vec::new();
        let mut offset = 0;
        let mut cur_block_len = 0;
        loop {
            let codepoint = read_codepoint(data, offset, codepoint_size);
            // null terminator
            if codepoint == 0 {
                blocks.push(TextDecoderBlock::Text(&data[offset - cur_block_len..offset]));
                break;
            }
            // escape sequences
            else if codepoint == 0x1A {
                blocks.push(TextDecoderBlock::Text(&data[offset - cur_block_len..offset]));
                cur_block_len = 0;

                let tag_len = u8::from_be(data[offset + codepoint_size]);
                blocks.push(TextDecoderBlock::EscapeSequence(
                    &data[offset + codepoint_size + 1..offset + tag_len as usize],
                ));
                offset += tag_len as usize;
            }
            // normal characters
            else {
                offset += codepoint_size;
                cur_block_len += codepoint_size;
            }
        }

        let decoder = match self {
            TextEncoding::Undefined | TextEncoding::CP1252 => WINDOWS_1252,
            TextEncoding::UTF8 => UTF_8,
            TextEncoding::UTF16 => UTF_16BE,
            TextEncoding::ShiftJIS => SHIFT_JIS,
        };
        let mut text = String::new();
        for block in blocks {
            match block {
                TextDecoderBlock::Text(bytes) => text.push_str(&decoder.decode(bytes).0),
                TextDecoderBlock::EscapeSequence(tag) => {
                    text.push('\u{1A}');
                    text.push_str(&format!("{}", tag.len()));
                    text.push_str("0x");
                    for b in tag {
                        text.push_str(&format!("{:02X}", b));
                    }
                }
            }
        }
        text
    }

    pub fn encode(&self, text: &str) -> Vec<u8> {
        let encoder = match self {
            TextEncoding::Undefined | TextEncoding::CP1252 => WINDOWS_1252,
            TextEncoding::UTF8 => UTF_8,
            TextEncoding::UTF16 => UTF_16BE,
            TextEncoding::ShiftJIS => SHIFT_JIS,
        };
        let mut out = Vec::new();
        let mut offset = 0;
        while offset < text.len() {
            if text[offset..].starts_with('\u{1A}') {
                let tag_start = text[offset..].find("0x").unwrap();
                // this is in BYTES, not characters, so sometimes we multiply by two when dealing with characters
                let tag_len: usize = text[offset + 1..offset + tag_start]
                    .parse()
                    .expect("Invalid tag length in BMG string");
                let tag_str = &text[offset + tag_start + 2..offset + tag_start + 2 + (tag_len * 2)];
                let tag_bytes = u64::from_str_radix(tag_str, 16).expect("Invalid digits in BMG text tag");
                out.push(0x1A);
                out.push((tag_len + 1 + self.codepoint_size()) as u8);
                out.extend(&tag_bytes.to_be_bytes()[8 - tag_len..]);
                offset += (tag_len * 2) + tag_start + 2;
            } else {
                let next_sub_index = text[offset..].find('\u{1A}').unwrap_or(text[offset..].len());
                out.extend(encoder.encode(&text[offset..offset + next_sub_index]).0.iter());
                offset += next_sub_index;
            }
        }
        out.push(b'\0');

        out
    }
}

#[derive(Debug)]
enum TextDecoderBlock<'a> {
    Text(&'a [u8]),
    EscapeSequence(&'a [u8]),
}

#[derive(Debug)]
struct TextIndexTable {
    section_size: u32, // bytes
    num_entries: u16,
    entry_size: u16,
    bmg_file_id: u16,
    default_color: u8,
    _unk1: u8,
    messages: Vec<TextIndexEntry>,
}

impl TextIndexTable {
    const MAGIC: &'static [u8] = b"INF1";
    const DRY_SIZE: usize = 0x10;

    pub fn new() -> TextIndexTable {
        TextIndexTable {
            section_size: TextIndexTable::DRY_SIZE as u32,
            num_entries: 0,
            entry_size: 4,
            bmg_file_id: 0,
            default_color: 0,
            _unk1: 0,
            messages: Vec::new(),
        }
    }

    pub fn add_message(&mut self, offset: u32, attributes: Vec<u8>) {
        self.num_entries += 1;
        self.entry_size = max(self.entry_size, attributes.len() as u16 + 4);
        self.section_size += self.entry_size as u32;
        self.messages.push(TextIndexEntry {
            text_offset: offset,
            attributes,
        })
    }

    pub fn write(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.section_size as usize);
        out.extend(TextIndexTable::MAGIC);
        out.extend(self.section_size.to_be_bytes());
        out.extend(self.num_entries.to_be_bytes());
        out.extend(self.entry_size.to_be_bytes());
        out.extend(self.bmg_file_id.to_be_bytes());
        out.push(self.default_color);
        out.push(self._unk1);
        out.extend(self.messages.iter().map(|entry| entry.write()).flatten());

        out
    }

    /// Assumes a TextIndexTable (INF1) section begins at index 0 of the given slice
    pub fn read(data: &[u8]) -> Result<TextIndexTable, BmgError> {
        if &data[..0x4] != TextIndexTable::MAGIC {
            return Err(BmgError::InvalidSectionMagic);
        }

        let section_length = read_u32(data, 0x4);
        let num_entries = read_u16(data, 0x8);
        let entry_size = read_u16(data, 0xA);
        let bmg_file_id = read_u16(data, 0xC);
        let default_color = data[0xE];
        let unk1 = data[0xF];
        let messages: Vec<TextIndexEntry> = data[0x10..section_length as usize]
            .chunks_exact(entry_size as usize)
            .take(num_entries as usize)
            .map(|chunk| TextIndexEntry::read(chunk, entry_size as usize))
            .collect();

        debug!(
            "Read TextIndexTable of size {} bytes and {} messages",
            section_length,
            &messages.len()
        );

        Ok(TextIndexTable {
            section_size: section_length,
            num_entries,
            entry_size,
            bmg_file_id,
            default_color,
            _unk1: unk1,
            messages,
        })
    }
}

#[derive(Debug)]
struct TextIndexEntry {
    /// Offset into the DAT1 text pool of the beginning of the referenced string
    text_offset: u32,
    attributes: Vec<u8>,
}

impl TextIndexEntry {
    pub fn write(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(4 + self.attributes.len());
        out.extend(self.text_offset.to_be_bytes());
        out.extend(&self.attributes);
        out
    }

    pub fn read(data: &[u8], len: usize) -> TextIndexEntry {
        TextIndexEntry {
            text_offset: read_u32(data, 0x0),
            attributes: Vec::from(&data[0x4..len]),
        }
    }
}

#[derive(Debug)]
struct StringPool {
    section_size: u32, // bytes
    /// Blob of null-terminated strings. Each character is either one or two bytes,
    /// determined by the text encoding in the header.
    strings: Vec<u8>,
}

impl StringPool {
    const MAGIC: &'static [u8] = b"DAT1";

    pub fn new() -> StringPool {
        StringPool {
            section_size: 8,
            strings: Vec::new(),
        }
    }

    pub fn add_message(&mut self, string: &[u8]) {
        self.section_size += string.len() as u32;
        self.strings.extend_from_slice(string);
    }

    pub fn write(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.section_size as usize);
        out.extend(StringPool::MAGIC);
        out.extend(self.section_size.to_be_bytes());
        out.extend(&self.strings);
        out
    }

    pub fn read(data: &[u8]) -> Result<StringPool, BmgError> {
        if &data[..0x4] != StringPool::MAGIC {
            return Err(BmgError::InvalidSectionMagic);
        }

        let section_size = read_u32(data, 0x4);
        let strings = data[0x8..section_size as usize].to_vec();

        debug!("Read StringPool of size {section_size} bytes");

        Ok(StringPool { section_size, strings })
    }
}

#[derive(Debug)]
struct MessageIdTable {
    section_size: u32, // bytes
    num_messages: u16,
    format: u8,
    info: u8,
    message_ids: Vec<MessageId>,
}

impl MessageIdTable {
    const MAGIC: &'static [u8] = b"MID1";
    const DRY_SIZE: usize = 16;

    pub fn new() -> MessageIdTable {
        MessageIdTable {
            section_size: MessageIdTable::DRY_SIZE as u32,
            num_messages: 0,
            format: 0,
            info: 0,
            message_ids: Vec::new(),
        }
    }

    pub fn add_message(&mut self, message_id: MessageId) {
        self.section_size += 4;
        self.num_messages += 1;
        self.message_ids.push(message_id);
    }

    pub fn write(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.section_size as usize);
        out.extend(MessageIdTable::MAGIC);
        out.extend(self.section_size.to_be_bytes());
        out.extend(self.num_messages.to_be_bytes());
        out.push(self.format);
        out.push(self.info);
        out.extend(0u32.to_be_bytes()); // Padding
        out.extend(self.message_ids.iter().flat_map(|id| id.write()));
        out
    }

    pub fn read(data: &[u8]) -> Result<MessageIdTable, BmgError> {
        if &data[..0x4] != MessageIdTable::MAGIC {
            return Err(BmgError::InvalidSectionMagic);
        }

        let section_size = read_u32(data, 0x4);
        let num_messages = read_u16(data, 0x8);
        let format = data[0xA];
        let info = data[0xB];
        let message_ids: Vec<MessageId> = data[0x10..section_size as usize]
            .chunks_exact(4)
            .map(|chunk| MessageId::read(&chunk))
            .collect();

        debug!(
            "Read MessageIdTable of size {} bytes and {} messages",
            section_size,
            message_ids.len()
        );

        Ok(MessageIdTable {
            section_size,
            num_messages,
            format,
            info,
            message_ids,
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MessageId {
    id: u32,
    sub_id: u8,
}

impl MessageId {
    pub fn write(&self) -> [u8; 4] {
        (self.id << 8 | self.sub_id as u32).to_be_bytes()
    }

    pub fn read(data: &[u8]) -> MessageId {
        let value = read_u32(data, 0);
        MessageId {
            id: value >> 8,
            sub_id: (value & 0xFF) as u8,
        }
    }
}

#[derive(Debug)]
struct UnknownSection {
    magic: [u8; 4],
    section_size: u32,
    data: Vec<u8>,
}

impl UnknownSection {
    pub fn write(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.section_size as usize);
        out.extend(self.magic);
        out.extend(self.section_size.to_be_bytes());
        out.extend(&self.data);
        out
    }

    pub fn read(data: &[u8]) -> Result<UnknownSection, BmgError> {
        let magic: [u8; 4] = data[..0x4].try_into().map_err(|_| BmgError::InvalidSectionMagic)?;
        let section_size = read_u32(data, 0x4);
        debug!(
            "Reading unknown section type with magic {} and size {} bytes",
            std::str::from_utf8(&magic).unwrap(),
            section_size
        );
        Ok(UnknownSection {
            magic,
            section_size,
            data: data[0x8..section_size as usize].to_vec(),
        })
    }
}

#[derive(Debug, Error)]
pub enum BmgError {
    #[error("Invalid magic byte sequence in BMG header. Expected \"{}\"", std::str::from_utf8(BmgHeader::MAGIC).unwrap())]
    InvalidHeaderMagic,

    #[error("Invalid magic byte sequence in BMG section")]
    InvalidSectionMagic,

    #[error("Unrecognized BMG text encoding byte '{0}'")]
    InvalidTextEncoding(u8),
}
