//! Minimal, dependency-free PDF page extractor for comic PDFs.
//!
//! Comic PDFs are almost always one full-page image per page. Rather than
//! linking a native PDF engine (which would add a platform-specific binary to
//! every Windows build), this module parses the PDF structure just far enough
//! to walk the page tree and pull out each page's image XObject stream. Text
//! PDFs are reported as unsupported rather than rendered incorrectly.

use std::collections::HashMap;
use std::io::Read;

use crate::error::{AppError, AppResult};

const MAX_STREAM_BYTES: usize = 256 * 1024 * 1024;

pub struct PdfPage {
    pub index: usize,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    /// File extension implied by the embedded image filter.
    pub extension: &'static str,
}

#[derive(Debug, Clone)]
enum PdfValue {
    Name(String),
    Number(f64),
    Ref(u32),
    Array(Vec<PdfValue>),
    Dict(HashMap<String, PdfValue>),
    Other,
}

impl PdfValue {
    fn as_dict(&self) -> Option<&HashMap<String, PdfValue>> {
        match self {
            PdfValue::Dict(dict) => Some(dict),
            _ => None,
        }
    }

    fn as_name(&self) -> Option<&str> {
        match self {
            PdfValue::Name(name) => Some(name),
            _ => None,
        }
    }

    fn as_number(&self) -> Option<f64> {
        match self {
            PdfValue::Number(value) => Some(*value),
            _ => None,
        }
    }

    fn as_ref(&self) -> Option<u32> {
        match self {
            PdfValue::Ref(id) => Some(*id),
            _ => None,
        }
    }
}

struct Object {
    value: PdfValue,
    stream: Option<Vec<u8>>,
}

pub fn read_pdf(path: &std::path::Path) -> AppResult<Vec<PdfPage>> {
    let bytes = std::fs::read(path)?;
    if !bytes.starts_with(b"%PDF") {
        return Err(AppError::Archive("not a PDF file".into()));
    }
    let objects = parse_objects(&bytes);

    let root = objects
        .values()
        .find_map(|object| object.value.as_dict())
        .and_then(|dict| dict.get("Type"))
        .and_then(|value| match value {
            PdfValue::Name(name) if name == "Catalog" => Some(()),
            _ => None,
        })
        .is_some();

    let catalog_pages = objects
        .values()
        .find_map(|object| {
            let dict = object.value.as_dict()?;
            if dict.get("Type").and_then(PdfValue::as_name) == Some("Catalog") {
                dict.get("Pages").and_then(PdfValue::as_ref)
            } else {
                None
            }
        })
        .ok_or_else(|| AppError::Archive("PDF has no page tree".into()))?;

    let mut order = Vec::new();
    let mut visited = std::collections::HashSet::new();
    collect_pages(&objects, catalog_pages, &mut order, &mut visited);

    let mut pages = Vec::new();
    for (index, page_ref) in order.iter().enumerate() {
        let Some(page) = objects.get(page_ref) else {
            continue;
        };
        let Some(dict) = page.value.as_dict() else {
            continue;
        };
        let Some((width, height, data, extension)) = page_image(&objects, dict)? else {
            continue;
        };
        pages.push(PdfPage {
            index,
            width,
            height,
            data,
            extension,
        });
    }

    if pages.is_empty() {
        let _ = root;
        return Err(AppError::Archive(
            "PDF contains no extractable page images (text-only PDFs are unsupported)".into(),
        ));
    }
    Ok(pages)
}

fn collect_pages(
    objects: &HashMap<u32, Object>,
    node_ref: u32,
    order: &mut Vec<u32>,
    visited: &mut std::collections::HashSet<u32>,
) {
    if !visited.insert(node_ref) {
        return;
    }
    let Some(object) = objects.get(&node_ref) else {
        return;
    };
    let Some(dict) = object.value.as_dict() else {
        return;
    };
    let kind = dict.get("Type").and_then(PdfValue::as_name).unwrap_or("");
    if kind == "Page" {
        order.push(node_ref);
        return;
    }
    if let Some(PdfValue::Array(kids)) = dict.get("Kids") {
        for kid in kids {
            if let Some(id) = kid.as_ref() {
                collect_pages(objects, id, order, visited);
            }
        }
    }
}

type PageImage = (u32, u32, Vec<u8>, &'static str);

fn page_image(objects: &HashMap<u32, Object>, page: &HashMap<String, PdfValue>) -> AppResult<Option<PageImage>> {
    let Some(PdfValue::Dict(resources)) = page.get("Resources") else {
        // Resources can also be an indirect reference.
        let Some(resources_id) = page.get("Resources").and_then(PdfValue::as_ref) else {
            return Ok(None);
        };
        let Some(resource_object) = objects.get(&resources_id) else {
            return Ok(None);
        };
        let Some(dict) = resource_object.value.as_dict() else {
            return Ok(None);
        };
        return page_image_from_resources(objects, dict);
    };
    page_image_from_resources(objects, resources)
}

fn page_image_from_resources(
    objects: &HashMap<u32, Object>,
    resources: &HashMap<String, PdfValue>,
) -> AppResult<Option<PageImage>> {
    let xobjects = match resources.get("XObject") {
        Some(PdfValue::Dict(dict)) => dict.clone(),
        Some(PdfValue::Ref(id)) => match objects.get(id).and_then(|object| object.value.as_dict()) {
            Some(dict) => dict.clone(),
            None => return Ok(None),
        },
        _ => return Ok(None),
    };

    let mut best: Option<PageImage> = None;
    for value in xobjects.values() {
        let Some(id) = value.as_ref() else {
            continue;
        };
        let Some(object) = objects.get(&id) else {
            continue;
        };
        let Some(dict) = object.value.as_dict() else {
            continue;
        };
        if dict.get("Subtype").and_then(PdfValue::as_name) != Some("Image") {
            continue;
        }
        let Some(stream) = object.stream.as_ref() else {
            continue;
        };
        let filter = dict.get("Filter").and_then(PdfValue::as_name).unwrap_or("");
        let (data, extension) = match filter {
            "DCTDecode" => (stream.clone(), "jpg"),
            "JPXDecode" => (stream.clone(), "jp2"),
            "FlateDecode" => match inflate(stream) {
                Some(raw) => {
                    if let Some(jpeg) = extract_jpeg(&raw) {
                        (jpeg, "jpg")
                    } else if let Some(png) = wrap_png(dict, &raw) {
                        (png, "png")
                    } else {
                        continue;
                    }
                }
                None => continue,
            },
            // Raw image data: only usable when it is already a known format.
            "" => {
                if let Some(jpeg) = extract_jpeg(stream) {
                    (jpeg, "jpg")
                } else {
                    continue;
                }
            }
            _ => continue,
        };
        let width = dict
            .get("Width")
            .and_then(PdfValue::as_number)
            .unwrap_or(0.0) as u32;
        let height = dict
            .get("Height")
            .and_then(PdfValue::as_number)
            .unwrap_or(0.0) as u32;
        let candidate = (width, height, data, extension);
        // Prefer the largest image on the page (the page art, not a logo).
        if best
            .as_ref()
            .is_none_or(|current| (current.0 as u64 * current.1 as u64) < (width as u64 * height as u64))
        {
            best = Some(candidate);
        }
    }
    Ok(best)
}

fn inflate(data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut decoder = flate2::read::ZlibDecoder::new(data);
    match decoder
        .by_ref()
        .take(MAX_STREAM_BYTES as u64)
        .read_to_end(&mut out)
    {
        Ok(_) if !out.is_empty() => Some(out),
        _ => None,
    }
}

fn extract_jpeg(data: &[u8]) -> Option<Vec<u8>> {
    let start = data.windows(2).position(|window| window == [0xFF, 0xD8])?;
    let end = data
        .windows(2)
        .rposition(|window| window == [0xFF, 0xD9])?;
    if end <= start {
        return None;
    }
    Some(data[start..=end + 1].to_vec())
}

fn wrap_png(dict: &HashMap<String, PdfValue>, raw: &[u8]) -> Option<Vec<u8>> {
    let width = dict.get("Width").and_then(PdfValue::as_number)? as u32;
    let height = dict.get("Height").and_then(PdfValue::as_number)? as u32;
    let color_space = dict.get("ColorSpace").and_then(PdfValue::as_name).unwrap_or("");
    let components: u8 = match color_space {
        "DeviceRGB" => 3,
        "DeviceGray" => 1,
        "DeviceCMYK" => 4,
        _ => return None,
    };
    if width == 0 || height == 0 {
        return None;
    }
    let expected = width as usize * height as usize * components as usize;
    if raw.len() < expected {
        return None;
    }
    let color_type = if components == 1 { 0 } else if components == 3 { 2 } else { 6 };
    Some(encode_png(width, height, color_type, &raw[..expected]))
}

fn encode_png(width: u32, height: u32, color_type: u8, data: &[u8]) -> Vec<u8> {
    let channels = match color_type {
        0 => 1usize,
        2 => 3,
        6 => 4,
        _ => 1,
    };
    let stride = width as usize * channels;
    let mut raw = Vec::with_capacity((stride + 1) * height as usize);
    for row in 0..height as usize {
        raw.push(0); // filter: none
        let start = row * stride;
        raw.extend_from_slice(&data[start..start + stride]);
    }

    let mut out = Vec::new();
    out.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, color_type, 0, 0, 0]);
    push_png_chunk(&mut out, b"IHDR", &ihdr);
    let mut compressor =
        flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    let _ = std::io::Write::write_all(&mut compressor, &raw);
    if let Ok(compressed) = compressor.finish() {
        push_png_chunk(&mut out, b"IDAT", &compressed);
    }
    push_png_chunk(&mut out, b"IEND", &[]);
    out
}

fn push_png_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc = crc32_init();
    crc = crc32_update(crc, kind);
    crc = crc32_update(crc, data);
    out.extend_from_slice(&(crc ^ 0xFFFF_FFFF).to_be_bytes());
}

fn crc32_init() -> u32 {
    0xFFFF_FFFF
}

fn crc32_update(mut crc: u32, data: &[u8]) -> u32 {
    for byte in data {
        crc ^= *byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                0xEDB8_8320 ^ (crc >> 1)
            } else {
                crc >> 1
            };
        }
    }
    crc
}

// --- Structure parsing -------------------------------------------------------

fn parse_objects(bytes: &[u8]) -> HashMap<u32, Object> {
    let mut objects = HashMap::new();
    let mut cursor = 0usize;
    while let Some(position) = find(&bytes[cursor..], b" obj") {
        let end_of_header = cursor + position;
        // Object headers are "num gen obj"; walk backwards over the generation
        // and then the object number.
        let mut start = end_of_header;
        let digits_start = |start: &mut usize| {
            while *start > 0 && bytes[*start - 1].is_ascii_digit() {
                *start -= 1;
            }
        };
        digits_start(&mut start);
        while start > 0 && bytes[start - 1].is_ascii_whitespace() {
            start -= 1;
        }
        digits_start(&mut start);
        let id: u32 = std::str::from_utf8(&bytes[start..end_of_header])
            .ok()
            .and_then(|text| text.split_whitespace().next())
            .and_then(|text| text.parse().ok())
            .unwrap_or(0);
        if id == 0 {
            cursor = end_of_header + 4;
            continue;
        }
        let body_start = end_of_header + 4;
        let (value, consumed) = parse_value(&bytes[body_start..]);
        let after_value = body_start + consumed;
        if bytes[after_value..]
            .iter()
            .skip_while(|byte| byte.is_ascii_whitespace())
            .take(6)
            .copied()
            .eq(b"stream".iter().copied())
        {
            let stream_keyword = after_value
                + bytes[after_value..]
                    .iter()
                    .position(|byte| *byte == b's')
                    .unwrap_or(0);
            let data_start = bytes[stream_keyword..]
                .iter()
                .position(|byte| *byte == b'\n')
                .map(|offset| stream_keyword + offset + 1)
                .unwrap_or(stream_keyword);
            let length = value
                .as_dict()
                .and_then(|dict| dict.get("Length"))
                .and_then(PdfValue::as_number)
                .map(|number| number as usize);
            let data_end = match length {
                Some(length) if data_start + length <= bytes.len() => data_start + length,
                _ => find(&bytes[data_start..], b"endstream")
                    .map(|offset| data_start + offset)
                    .unwrap_or(bytes.len()),
            };
            let raw = bytes[data_start..data_end].to_vec();
            objects.insert(id, Object { value, stream: Some(raw) });
            cursor = data_end;
            continue;
        };
        objects.insert(id, Object { value, stream: None });
        cursor = after_value.max(end_of_header + 4);
    }
    objects
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn parse_value(input: &[u8]) -> (PdfValue, usize) {
    let mut index = 0;
    skip_ws(input, &mut index);
    let (value, _) = parse_value_at(input, &mut index);
    (value, index)
}

fn skip_ws(input: &[u8], index: &mut usize) {
    while *index < input.len() && input[*index].is_ascii_whitespace() {
        *index += 1;
    }
}

fn parse_value_at(input: &[u8], index: &mut usize) -> (PdfValue, usize) {
    skip_ws(input, index);
    if *index >= input.len() {
        return (PdfValue::Other, 0);
    }
    let start = *index;
    match input[*index] {
        b'<' if input.get(*index + 1) == Some(&b'<') => {
            *index += 2;
            let mut dict = HashMap::new();
            loop {
                skip_ws(input, index);
                if *index >= input.len() {
                    break;
                }
                if input[*index] == b'>' && input.get(*index + 1) == Some(&b'>') {
                    *index += 2;
                    break;
                }
                let (key, _) = parse_value_at(input, index);
                let PdfValue::Name(key) = key else {
                    // Skip malformed entries rather than aborting the file.
                    *index += 1;
                    continue;
                };
                skip_ws(input, index);
                let (value, _) = parse_value_at(input, index);
                dict.insert(key, value);
            }
            (PdfValue::Dict(dict), *index - start)
        }
        b'[' => {
            *index += 1;
            let mut items = Vec::new();
            loop {
                skip_ws(input, index);
                if *index >= input.len() {
                    break;
                }
                if input[*index] == b']' {
                    *index += 1;
                    break;
                }
                let (value, _) = parse_value_at(input, index);
                if matches!(value, PdfValue::Other) {
                    *index += 1;
                    continue;
                }
                items.push(value);
            }
            (PdfValue::Array(items), *index - start)
        }
        b'/' => {
            *index += 1;
            let begin = *index;
            while *index < input.len()
                && !input[*index].is_ascii_whitespace()
                && !matches!(input[*index], b'<' | b'>' | b'[' | b']' | b'/' | b'(')
            {
                *index += 1;
            }
            let name = String::from_utf8_lossy(&input[begin..*index]).to_string();
            (PdfValue::Name(name), *index - start)
        }
        b'(' => {
            // Skip literal strings entirely; they never carry our structure.
            let mut depth = 0usize;
            while *index < input.len() {
                match input[*index] {
                    b'\\' => *index += 2,
                    b'(' => {
                        depth += 1;
                        *index += 1;
                    }
                    b')' => {
                        depth -= 1;
                        *index += 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => *index += 1,
                }
            }
            (PdfValue::Other, *index - start)
        }
        byte if byte.is_ascii_digit() || byte == b'-' || byte == b'+' || byte == b'.' => {
            let begin = *index;
            while *index < input.len()
                && (input[*index].is_ascii_digit()
                    || matches!(input[*index], b'-' | b'+' | b'.'))
            {
                *index += 1;
            }
            let text = std::str::from_utf8(&input[begin..*index]).unwrap_or("0");
            let number = text.parse::<f64>().unwrap_or(0.0);
            // Look ahead for a "gen R" reference (e.g. "12 0 R").
            let after_number = *index;
            let mut lookahead = after_number;
            skip_ws(input, &mut lookahead);
            let gen_start = lookahead;
            while lookahead < input.len() && input[lookahead].is_ascii_digit() {
                lookahead += 1;
            }
            if lookahead > gen_start {
                let mut after_gen = lookahead;
                skip_ws(input, &mut after_gen);
                if input.get(after_gen) == Some(&b'R')
                    && input
                        .get(after_gen + 1)
                        .is_none_or(|byte| byte.is_ascii_whitespace() || b"<>[]/%".contains(byte))
                {
                    *index = after_gen + 1;
                    return (PdfValue::Ref(number as u32), *index - start);
                }
            }
            (PdfValue::Number(number), *index - start)
        }
        _ => {
            // Bare keyword (obj/endobj/stream/true/false/null/...).
            while *index < input.len() && !input[*index].is_ascii_whitespace() {
                *index += 1;
            }
            if *index == start {
                *index += 1;
            }
            (PdfValue::Other, *index - start)
        }
    }
}
