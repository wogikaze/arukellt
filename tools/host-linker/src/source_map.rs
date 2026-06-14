//! Parse `metadata.debug.source_map` custom sections from emitted Wasm.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMapEntry {
    pub code_offset: u32,
    pub source_line: u32,
}

pub fn parse_source_map(wasm_bytes: &[u8]) -> Vec<SourceMapEntry> {
    let mut entries = Vec::new();
    let Some(payload) = find_custom_section_payload(wasm_bytes, b"metadata.debug.source_map") else {
        return entries;
    };
    let mut pos = 0usize;
    let version = read_leb_u32(payload, &mut pos);
    if version != 1 {
        return entries;
    }
    let count = read_leb_u32(payload, &mut pos) as usize;
    for _ in 0..count {
        entries.push(SourceMapEntry {
            code_offset: read_leb_u32(payload, &mut pos),
            source_line: read_leb_u32(payload, &mut pos),
        });
    }
    entries.sort_by_key(|e| e.code_offset);
    entries
}

pub fn line_to_code_offset(entries: &[SourceMapEntry], line: u32) -> Option<u32> {
    entries
        .iter()
        .filter(|e| e.source_line <= line)
        .max_by_key(|e| e.source_line)
        .map(|e| e.code_offset)
}

fn find_custom_section_payload<'a>(wasm: &'a [u8], name: &[u8]) -> Option<&'a [u8]> {
    if wasm.len() < 8 || &wasm[0..4] != b"\0asm" {
        return None;
    }
    let mut pos = 8usize;
    while pos < wasm.len() {
        let section_id = wasm[pos];
        pos += 1;
        let (section_len, header_len) = read_leb_usize(wasm, pos)?;
        pos += header_len;
        let section_end = pos.checked_add(section_len)?;
        if section_end > wasm.len() {
            return None;
        }
        if section_id == 0 {
            let mut inner = pos;
            let (name_len, name_hdr) = read_leb_usize(wasm, inner)?;
            inner += name_hdr;
            if inner + name_len > section_end {
                return None;
            }
            if &wasm[inner..inner + name_len] == name {
                return Some(&wasm[inner + name_len..section_end]);
            }
        }
        pos = section_end;
    }
    None
}

fn read_leb_u32(data: &[u8], pos: &mut usize) -> u32 {
    let (v, n) = read_leb_usize(data, *pos).unwrap_or((0, 0));
    *pos += n;
    v as u32
}

fn read_leb_usize(data: &[u8], start: usize) -> Option<(usize, usize)> {
    let mut result = 0usize;
    let mut shift = 0u32;
    for (i, &byte) in data[start..].iter().enumerate() {
        result |= ((byte & 0x7f) as usize) << shift;
        if byte & 0x80 == 0 {
            return Some((result, i + 1));
        }
        shift += 7;
        if shift > 35 {
            return None;
        }
    }
    None
}
