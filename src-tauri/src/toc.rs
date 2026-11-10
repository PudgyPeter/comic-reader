//! Table-of-contents detection for omnibus comics.
//!
//! A collected edition usually has a contents page listing each chapter and the
//! page it starts on. Rather than hard-coding a layout, this runs OCR over a
//! page and then looks for rows of the shape "Chapter N — Title ... page".
//!
//! Parsing is deliberately defensive: OCR output is noisy, page numbers printed
//! in a book are almost never the same as archive page indices (front matter,
//! covers, blanks), and a wrong split is worse than no split. The caller always
//! shows the proposed ranges for the user to confirm before anything is written.

use crate::error::{AppError, AppResult};
use crate::models::TocEntry;

/// One chapter row recovered from a contents page.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedEntry {
    pub title: String,
    pub printed_page: i64,
}

/// OCR a single page image into text lines.
fn ocr_lines(image_bytes: &[u8], models: Option<&OcrModels>) -> AppResult<Vec<String>> {
    let models = models.ok_or_else(|| {
        AppError::message("OCR models are unavailable in this build")
    })?;
    let image = image::load_from_memory(image_bytes)
        .map_err(|err| AppError::message(format!("cannot decode page image: {err}")))?
        .into_rgb8();
    let engine = ocrs::OcrEngine::new(ocrs::OcrEngineParams {
        detection_model: Some(models.detection()?),
        recognition_model: Some(models.recognition()?),
        ..Default::default()
    })
    .map_err(|err| AppError::message(format!("OCR engine failed: {err}")))?;
    let source = ocrs::ImageSource::from_bytes(image.as_raw(), image.dimensions())
        .map_err(|err| AppError::message(format!("OCR input rejected: {err}")))?;
    let input = engine
        .prepare_input(source)
        .map_err(|err| AppError::message(format!("OCR prepare failed: {err}")))?;
    let words = engine
        .detect_words(&input)
        .map_err(|err| AppError::message(format!("OCR detection failed: {err}")))?;
    let lines = engine.find_text_lines(&input, &words);

    // A contents page is often two columns ("Chapter One ... 9") and OCR splits
    // title and number into separate lines. Rebuild rows geometrically: group
    // text lines whose vertical centres overlap, then join them left to right.
    let mut rows: Vec<(f32, Vec<(f32, String)>)> = Vec::new();
    let recognized = engine
        .recognize_text(&input, &lines)
        .map_err(|err| AppError::message(format!("OCR recognition failed: {err}")))?;
    for (word_rects, text) in lines.iter().zip(recognized) {
        let Some(text) = text else { continue };
        let label = text.to_string().trim().to_string();
        // Single digits are legitimate page numbers ("9"), so only discard
        // single non-numeric characters, which are OCR noise.
        if label.is_empty() || (label.len() == 1 && !label.chars().all(|c| c.is_ascii_digit())) {
            continue;
        }
        let Some(bounds) = rten_imageproc::bounding_rect(word_rects.iter()) else {
            continue;
        };
        let centre_y = (bounds.top() + bounds.bottom()) / 2.0;
        let height = (bounds.bottom() - bounds.top()).max(1.0);
        match rows.iter_mut().find(|(row_y, _)| (row_y - centre_y).abs() < height * 0.6) {
            Some((_, items)) => items.push((bounds.left(), label)),
            None => rows.push((centre_y, vec![(bounds.left(), label)])),
        }
    }
    rows.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    Ok(rows
        .into_iter()
        .map(|(_, mut items)| {
            items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            items
                .into_iter()
                .map(|(_, text)| text)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect())
}

/// Lazily loads the bundled OCR models. They are ~12 MB of ONNX, so they load
/// once per split request rather than at startup.
pub struct OcrModels {
    detection: std::path::PathBuf,
    recognition: std::path::PathBuf,
}

impl OcrModels {
    /// `dir` is the models directory (Tauri resource dir in production, the
    /// crate `models/` folder in tests).
    pub fn new(dir: &std::path::Path) -> Self {
        Self {
            detection: dir.join("text-detection.onnx"),
            recognition: dir.join("text-recognition.onnx"),
        }
    }

    pub fn available(&self) -> bool {
        self.detection.is_file() && self.recognition.is_file()
    }

    fn detection(&self) -> AppResult<rten::Model> {
        let bytes = std::fs::read(&self.detection)?;
        rten::Model::load(bytes)
            .map_err(|err| AppError::message(format!("cannot load detection model: {err}")))
    }

    fn recognition(&self) -> AppResult<rten::Model> {
        let bytes = std::fs::read(&self.recognition)?;
        rten::Model::load(bytes)
            .map_err(|err| AppError::message(format!("cannot load recognition model: {err}")))
    }
}

/// True when a contents page is present, judged by OCR noise we will parse
/// anyway. Used to skip pages that clearly are not a table of contents.
fn looks_like_contents(lines: &[String]) -> bool {
    let joined = lines.join(" ").to_lowercase();
    let has_heading = ["contents", "table of contents", "in this volume", "this volume contains"]
        .iter()
        .any(|word| joined.contains(word));
    let chapter_rows = lines.iter().filter(|line| parse_row(line).is_some()).count();
    has_heading || chapter_rows >= 2
}

/// Parses one OCR line into a chapter row.
///
/// Accepted shapes (OCR-tolerant):
///   "Chapter 1  Fallout   9"
///   "Chapter One - Fallout  9"
///   "Sonic the Hedgehog #21 .... 45"
///   "1. Fallout  9"
fn parse_row(raw: &str) -> Option<ParsedEntry> {
    let line = raw.trim();
    if line.len() < 3 {
        return None;
    }
    let lower = line.to_lowercase();
    let chapterish = lower.contains("chapter")
        || lower.contains("issue")
        || lower.contains("part")
        || lower.contains("episode")
        || line
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_digit());
    if !chapterish {
        return None;
    }

    // Trailing page number, allowing leader dots (".... 45").
    let (body, page_text) = split_trailing_number(line)?;
    let printed_page = page_text.parse::<i64>().ok()?;
    if printed_page <= 0 || printed_page > 5000 {
        return None;
    }

    let title = normalize_title(body);
    if title.is_empty() {
        return None;
    }
    Some(ParsedEntry {
        title,
        printed_page,
    })
}

/// Splits "Chapter 1 Fallout 9" into ("Chapter 1 Fallout", "9"). Requires at
/// least one space so a bare number line is rejected.
fn split_trailing_number(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim_end_matches(|c: char| c == '.' || c.is_whitespace());
    let digits_start = trimmed
        .char_indices()
        .rev()
        .take_while(|(_, c)| c.is_ascii_digit())
        .last()
        .map(|(index, _)| index)?;
    if digits_start == 0 {
        return None;
    }
    let body = trimmed[..digits_start].trim_end_matches(|c: char| {
        c == '.' || c == '-' || c == '—' || c == '–' || c.is_whitespace()
    });
    if body.is_empty() {
        return None;
    }
    Some((body, &trimmed[digits_start..]))
}

/// Cleans OCR noise out of a chapter title.
fn normalize_title(body: &str) -> String {
    let cleaned = body
        .trim_matches(|c: char| c == '.' || c == ':' || c == '-' || c == '—' || c.is_whitespace())
        .replace("  ", " ");
    cleaned.trim().to_string()
}

/// Extracts chapter rows from OCRed lines, in printed order.
pub fn parse_contents(lines: &[String]) -> Vec<ParsedEntry> {
    lines.iter().filter_map(|line| parse_row(line)).collect()
}

/// Converts printed page numbers into archive-relative page ranges.
///
/// Printed numbering and archive indices differ by a constant offset (covers and
/// front matter). `first_body_page` is the archive index that printed page 1
/// corresponds to; the caller derives it by matching the first chapter, or
/// passes the offset the user corrected in the UI.
pub fn ranges_from_entries(
    entries: &[ParsedEntry],
    total_pages: i64,
    first_body_page: i64,
) -> Vec<(ParsedEntry, i64, i64)> {
    let mut out = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let start = first_body_page + (entry.printed_page - 1);
        let end = entries
            .get(index + 1)
            .map(|next| first_body_page + (next.printed_page - 1))
            .unwrap_or(total_pages);
        let start = start.clamp(0, total_pages);
        let end = end.clamp(start, total_pages);
        out.push((entry.clone(), start, end));
    }
    out
}

/// Best-guess offset between printed page numbers and archive indices.
///
/// Most collected editions put the first chapter right after the contents page,
/// so a printed "1" usually maps a small number of pages in. We estimate from
/// the first entry's printed page and the page the TOC was found on.
pub fn guess_offset(first_printed: i64, toc_page: i64, total_pages: i64) -> i64 {
    if first_printed <= 1 {
        return (toc_page + 1).clamp(0, total_pages.max(0));
    }
    // Assume the TOC sits just before the first chapter.
    (toc_page + 1 - (first_printed - 1)).max(0)
}

/// Full detection pass: OCR candidate pages and return the best contents parse.
pub fn detect(
    page_loader: impl Fn(usize) -> AppResult<Vec<u8>>,
    total_pages: i64,
    models: Option<&OcrModels>,
) -> AppResult<(usize, Vec<ParsedEntry>)> {
    // Contents pages live near the front; scanning the first pages keeps a
    // 400-page omnibus from OCRing 400 pages for nothing.
    let limit = total_pages.clamp(0, 12) as usize;
    let mut best: Option<(usize, Vec<ParsedEntry>)> = None;
    for index in 0..limit {
        let bytes = match page_loader(index) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        let lines = match ocr_lines(&bytes, models) {
            Ok(lines) => lines,
            Err(_) => continue,
        };
        if !looks_like_contents(&lines) {
            continue;
        }
        let entries = parse_contents(&lines);
        if entries.len() >= 2 {
            let better = best
                .as_ref()
                .is_none_or(|(_, current)| entries.len() > current.len());
            if better {
                best = Some((index, entries));
            }
            // A contents page is normally the first match near the front. Stop
            // once it is plausible rather than OCRing every early page.
            if best.as_ref().is_some_and(|(_, found)| found.len() >= 2) {
                break;
            }
        }
    }
    best.ok_or_else(|| {
        AppError::not_found("a contents page (needs OCR models and a chapter list page)")
    })
}

impl From<ParsedEntry> for TocEntry {
    fn from(entry: ParsedEntry) -> Self {
        TocEntry {
            title: entry.title,
            printed_page: entry.printed_page,
        }
    }
}
