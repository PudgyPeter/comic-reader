use std::io::{Cursor, Read};

use image::ImageReader;
use serde::Serialize;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveEntry {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

pub const MAX_ENTRY_BYTES: u64 = 128 * 1024 * 1024;

pub fn is_image_path(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [".jpg", ".jpeg", ".png", ".webp", ".gif", ".bmp", ".avif"]
        .iter()
        .any(|ext| lower.ends_with(ext))
}

pub fn natural_key(name: &str) -> Vec<NaturalPart> {
    let mut parts = Vec::new();
    let mut buffer = String::new();
    let mut numeric = false;
    for ch in name.chars() {
        if ch.is_ascii_digit() {
            if !numeric && !buffer.is_empty() {
                parts.push(NaturalPart::Text(buffer.to_lowercase()));
                buffer.clear();
            }
            numeric = true;
            buffer.push(ch);
        } else {
            if numeric && !buffer.is_empty() {
                parts.push(NaturalPart::Number(
                    buffer.trim_start_matches('0').parse::<u64>().unwrap_or(0),
                ));
                buffer.clear();
            }
            numeric = false;
            buffer.push(ch);
        }
    }
    if !buffer.is_empty() {
        if numeric {
            parts.push(NaturalPart::Number(
                buffer.trim_start_matches('0').parse::<u64>().unwrap_or(0),
            ));
        } else {
            parts.push(NaturalPart::Text(buffer.to_lowercase()));
        }
    }
    parts
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NaturalPart {
    Text(String),
    Number(u64),
}

pub fn read_archive(path: &std::path::Path) -> AppResult<Vec<ArchiveEntry>> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut archive = zip::ZipArchive::new(reader)?;
    let mut names = Vec::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }
        let Some(enclosed) = entry.enclosed_name() else {
            continue;
        };
        let name = enclosed.to_string_lossy().to_string();
        if name.starts_with("__MACOSX/") {
            continue;
        }
        if !is_image_path(&name) {
            continue;
        }
        if entry.size() > MAX_ENTRY_BYTES {
            continue;
        }
        names.push((name, index));
    }
    names.sort_by(|a, b| {
        natural_key(&a.0)
            .cmp(&natural_key(&b.0))
            .then_with(|| a.0.cmp(&b.0))
    });

    let mut out = Vec::with_capacity(names.len());
    for (name, index) in names {
        let mut entry = archive.by_index(index)?;
        let mut data = Vec::with_capacity(entry.size() as usize);
        entry.by_ref().take(MAX_ENTRY_BYTES).read_to_end(&mut data)?;
        let (width, height) = dimensions(&data).unwrap_or((0, 0));
        out.push(ArchiveEntry {
            name,
            width,
            height,
            data,
        });
    }
    if out.is_empty() {
        return Err(AppError::Archive(format!(
            "no readable image pages in {}",
            path.display()
        )));
    }
    Ok(out)
}

/// PDF pages are converted into the same in-memory page shape as archives.
pub fn read_pdf_entries(path: &std::path::Path) -> AppResult<Vec<ArchiveEntry>> {
    let pages = crate::pdf::read_pdf(path)?;
    let mut out = Vec::with_capacity(pages.len());
    for page in pages {
        let (width, height) = dimensions(&page.data).unwrap_or((page.width, page.height));
        out.push(ArchiveEntry {
            name: format!("page-{:04}.{}", page.index + 1, page.extension),
            width,
            height,
            data: page.data,
        });
    }
    Ok(out)
}

pub fn read_rar(path: &std::path::Path) -> AppResult<Vec<ArchiveEntry>> {
    let file = std::fs::File::open(path)?;
    let mut archive = unrar_rs::RarArchive::open(std::io::BufReader::new(file))
        .map_err(|err| AppError::Archive(format!("cannot open rar: {err}")))?;

    let mut names = Vec::new();
    for index in 0..archive.len() {
        let Ok(entry) = archive.by_index(index) else {
            continue;
        };
        let info = entry.info().clone();
        drop(entry);
        if info.is_directory {
            continue;
        }
        let name = info.name.clone();
        if !is_image_path(&name) || name.starts_with("__MACOSX/") {
            continue;
        }
        if info.unpacked_size.is_some_and(|size| size > MAX_ENTRY_BYTES) {
            continue;
        }
        names.push((name, index));
    }
    names.sort_by(|a, b| {
        natural_key(&a.0)
            .cmp(&natural_key(&b.0))
            .then_with(|| a.0.cmp(&b.0))
    });

    let mut out = Vec::with_capacity(names.len());
    for (name, index) in names {
        let entry = archive
            .by_index(index)
            .map_err(|err| AppError::Archive(format!("cannot read rar member: {err}")))?;
        let mut data = Vec::new();
        entry
            .copy_to(&mut data)
            .map_err(|err| AppError::Archive(format!("cannot extract rar member: {err}")))?;
        if data.len() as u64 > MAX_ENTRY_BYTES {
            continue;
        }
        let (width, height) = dimensions(&data).unwrap_or((0, 0));
        out.push(ArchiveEntry {
            name,
            width,
            height,
            data,
        });
    }
    if out.is_empty() {
        return Err(AppError::Archive(format!(
            "no readable image pages in {}",
            path.display()
        )));
    }
    Ok(out)
}

pub fn read_folder(path: &std::path::Path) -> AppResult<Vec<ArchiveEntry>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name.starts_with('$') {
            continue;
        }
        if is_image_path(&name) {
            files.push((name, entry.path()));
        }
    }
    files.sort_by_key(|entry| natural_key(&entry.0));
    let mut out = Vec::with_capacity(files.len());
    for (name, path) in files {
        let size = std::fs::metadata(&path)?.len();
        if size > MAX_ENTRY_BYTES {
            continue;
        }
        let data = std::fs::read(&path)?;
        let (width, height) = dimensions(&data).unwrap_or((0, 0));
        out.push(ArchiveEntry {
            name,
            width,
            height,
            data,
        });
    }
    if out.is_empty() {
        return Err(AppError::Archive(format!(
            "no readable images in {}",
            path.display()
        )));
    }
    Ok(out)
}

pub fn page_bytes(path: &std::path::Path, format: &str, entry_name: &str) -> AppResult<Vec<u8>> {
    match format {
        "folder" => {
            let candidate = path.join(entry_name);
            if !candidate.is_file() {
                return Err(AppError::not_found(format!("page {entry_name}")));
            }
            let size = std::fs::metadata(&candidate)?.len();
            if size > MAX_ENTRY_BYTES {
                return Err(AppError::invalid("page exceeds size limit"));
            }
            Ok(std::fs::read(candidate)?)
        }
        "cbz" | "zip" => {
            let file = std::fs::File::open(path)?;
            let reader = std::io::BufReader::new(file);
            let mut archive = zip::ZipArchive::new(reader)?;
            let mut entry = archive.by_name(entry_name)?;
            if entry.size() > MAX_ENTRY_BYTES {
                return Err(AppError::invalid("page exceeds size limit"));
            }
            let mut data = Vec::with_capacity(entry.size() as usize);
            entry.by_ref().take(MAX_ENTRY_BYTES).read_to_end(&mut data)?;
            Ok(data)
        }
        "cbr" => {
            let file = std::fs::File::open(path)?;
            let mut archive = unrar_rs::RarArchive::open(std::io::BufReader::new(file))
                .map_err(|err| AppError::Archive(format!("cannot open rar: {err}")))?;
            let entry = archive
                .by_name(entry_name)
                .map_err(|err| AppError::Archive(format!("cannot read rar member: {err}")))?;
            let mut data = Vec::new();
            entry
                .copy_to(&mut data)
                .map_err(|err| AppError::Archive(format!("cannot extract rar member: {err}")))?;
            if data.len() as u64 > MAX_ENTRY_BYTES {
                return Err(AppError::invalid("page exceeds size limit"));
            }
            Ok(data)
        }
        "pdf" => {
            // PDF pages are re-extracted on demand; the entry name encodes the
            // page index, so we only need that page rather than all of them.
            let index = entry_name
                .trim_start_matches("page-")
                .split('.')
                .next()
                .and_then(|value| value.parse::<usize>().ok())
                .ok_or_else(|| AppError::not_found(format!("page {entry_name}")))?;
            let pages = crate::pdf::read_pdf(path)?;
            pages
                .into_iter()
                .find(|page| page.index + 1 == index)
                .map(|page| page.data)
                .ok_or_else(|| AppError::not_found(format!("page {entry_name}")))
        }
        other => Err(AppError::invalid(format!("unsupported format: {other}"))),
    }
}

pub fn dimensions(data: &[u8]) -> Option<(u32, u32)> {
    let reader = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .ok()?;
    reader.into_dimensions().ok()
}

pub fn thumbnail(data: &[u8], max_width: u32) -> AppResult<Vec<u8>> {
    let reader = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|err| AppError::message(format!("image probe failed: {err}")))?;
    let image = reader
        .decode()
        .map_err(|err| AppError::message(format!("image decode failed: {err}")))?;
    let ratio = max_width as f32 / image.width().max(1) as f32;
    let height = ((image.height() as f32) * ratio).round().max(1.0) as u32;
    let resized = image.thumbnail(max_width, height.max(1));
    let mut out = Cursor::new(Vec::new());
    resized
        .write_to(&mut out, image::ImageFormat::Jpeg)
        .map_err(|err| AppError::message(format!("thumbnail encode failed: {err}")))?;
    Ok(out.into_inner())
}
