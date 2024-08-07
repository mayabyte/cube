use encoding_rs::SHIFT_JIS;
use std::{borrow::Cow, num::ParseIntError};

pub fn read_u16(data: &[u8], offset: u32) -> u16 {
    u16::from_be_bytes(data[offset as usize..offset as usize + 2].try_into().unwrap())
}

pub fn read_u32(data: &[u8], offset: u32) -> u32 {
    u32::from_be_bytes(data[offset as usize..offset as usize + 4].try_into().unwrap())
}

pub fn read_u64(data: &[u8], offset: u32) -> u64 {
    u64::from_be_bytes(data[offset as usize..offset as usize + 8].try_into().unwrap())
}

pub fn read_str(data: &[u8], offset: u32, len: u32) -> Cow<'_, str> {
    SHIFT_JIS.decode(&data[offset as usize..(offset + len) as usize]).0
}

pub fn read_str_until_null(data: &[u8], offset: u32) -> Cow<'_, str> {
    let mut i = 0;
    while data[offset as usize + i] != b"\0"[0] {
        i += 1;
    }
    read_str(data, offset, i as u32)
}

pub fn to_hex_string(bytes: &[u8]) -> String {
    let mut out = String::new();
    for b in bytes {
        out.push_str(&format!("{:02X}", b));
    }
    out
}

pub fn from_hex_string(string: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..string.len() / 2)
        .into_iter()
        .map(|idx| u8::from_str_radix(&string[idx * 2..(idx * 2) + 2], 16))
        .collect()
}

pub fn pad_to<const N: usize>(buf: &mut Vec<u8>) {
    while buf.len() % N != 0 {
        buf.push(0);
    }
}

pub fn padded_index_to<const N: u32>(idx: u32) -> u32 {
    (idx + (N - 1)) & !(N - 1)
}
