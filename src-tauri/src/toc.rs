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
    /// Full printed label, e.g. "Sonic the Hedgehog #35".
    pub title: String,
    /// Series the row belongs to, when the row names one. Collected editions
    /// routinely mix mini-series with the main run, so this is what lets a
    /// single file split into several series.
    pub series: Option<String>,
    /// Issue number as printed.
    pub number: String,
    pub printed_page: i64,
}

/// A recognised piece of text with its position on the page.
#[derive(Debug, Clone)]
pub struct TextBlock {
    left: f32,
    right: f32,
    centre_y: f32,
    height: f32,
    text: String,
}

impl TextBlock {
    pub fn new(left: f32, right: f32, centre_y: f32, height: f32, text: String) -> Self {
        Self {
            left,
            right,
            centre_y,
            height,
            text,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn debug_line(&self) -> String {
        format!(
            "x {:.0}-{:.0}  y {:.0}  h {:.0}  {:?}",
            self.left, self.right, self.centre_y, self.height, self.text
        )
    }
}

/// OCR a page into positioned text blocks.
pub fn ocr_blocks_for_probe(
    image_bytes: &[u8],
    models: &OcrModels,
) -> AppResult<Vec<TextBlock>> {
    ocr_blocks(image_bytes, Some(models))
}

/// Re-reads a short numeric block from a magnified crop.
///
/// Contents pages print page numbers inside small badges, often two or three
/// digits wide at thumbnail-ish sizes inside the page. At full-page scale the
/// recogniser drops leading digits (`138` reads as `38`) or turns the whole
/// badge into `1`. Cropping the badge and scaling it up recovers them cheaply,
/// because the crop is a few dozen pixels.
fn reread_number(
    image: &image::RgbImage,
    block: &TextBlock,
    engine: &ocrs::OcrEngine,
) -> Option<i64> {
    let pad_x = (block.right - block.left) * 0.45 + 6.0;
    let pad_y = block.height * 0.6 + 4.0;
    let left = (block.left - pad_x).max(0.0) as u32;
    let top = (block.centre_y - block.height / 2.0 - pad_y).max(0.0) as u32;
    let right = ((block.right + pad_x) as u32).min(image.width());
    let bottom = ((block.centre_y + block.height / 2.0 + pad_y) as u32).min(image.height());
    if right <= left + 2 || bottom <= top + 2 {
        return None;
    }
    let crop = image::imageops::crop_imm(image, left, top, right - left, bottom - top).to_image();
    let scale = 4u32;
    let scaled = image::imageops::resize(
        &crop,
        crop.width() * scale,
        crop.height() * scale,
        image::imageops::FilterType::Lanczos3,
    );
    let source = ocrs::ImageSource::from_bytes(scaled.as_raw(), scaled.dimensions()).ok()?;
    let input = engine.prepare_input(source).ok()?;
    let words = engine.detect_words(&input).ok()?;
    let lines = engine.find_text_lines(&input, &words);
    let recognized = engine.recognize_text(&input, &lines).ok()?;
    let joined: String = recognized
        .into_iter()
        .flatten()
        .map(|text| text.to_string())
        .collect::<Vec<_>>()
        .join("");
    let digits: String = joined.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() || digits.len() > 4 {
        return None;
    }
    let value = digits.parse::<i64>().ok()?;
    (1..=5000).contains(&value).then_some(value)
}

/// Re-reads a narrow strip of the page at higher magnification.
///
/// Badges printed between the columns of a contents page are small and sit on
/// art, so the full-page pass silently drops some of them. Recovering every
/// badge matters: a missing page number removes a chapter boundary, and the
/// split would then start a chapter too late and leave the previous one long.
///
/// `x0..x1` is the strip in page coordinates; results come back in the same
/// coordinate space so they can be merged with the full-page pass.
fn ocr_strip(
    image: &image::RgbImage,
    engine: &ocrs::OcrEngine,
    x0: u32,
    x1: u32,
) -> Vec<TextBlock> {
    let x0 = x0.min(image.width());
    let x1 = x1.min(image.width());
    if x1 <= x0 + 4 {
        return Vec::new();
    }
    // A whole-page-tall, badge-width crop is too thin for the detector, which
    // then returns nothing at all. Read it as overlapping vertical tiles and
    // drop the duplicates the overlap produces.
    const TILE: u32 = 900;
    const OVERLAP: u32 = 120;
    let scale = 3u32;
    let mut out: Vec<TextBlock> = Vec::new();
    let mut top = 0u32;
    while top < image.height() {
        let height = TILE.min(image.height() - top);
        let crop = image::imageops::crop_imm(image, x0, top, x1 - x0, height).to_image();
        let scaled = image::imageops::resize(
            &crop,
            crop.width() * scale,
            crop.height() * scale,
            image::imageops::FilterType::Lanczos3,
        );
        let parsed = ocrs::ImageSource::from_bytes(scaled.as_raw(), scaled.dimensions())
            .ok()
            .and_then(|source| engine.prepare_input(source).ok())
            .and_then(|input| {
                engine
                    .detect_words(&input)
                    .ok()
                    .map(|words| (input, words))
            })
            .and_then(|(input, words)| {
                let lines = engine.find_text_lines(&input, &words);
                engine.recognize_text(&input, &lines).ok().map(|recognized| (lines, recognized))
            });
        if let Some((lines, recognized)) = parsed {
                        for (word_rects, text) in lines.iter().zip(recognized) {
                            let Some(text) = text else { continue };
                            let label = text.to_string().trim().to_string();
                            let Some(bounds) = rten_imageproc::bounding_rect(word_rects.iter())
                            else {
                                continue;
                            };
                            let centre_y =
                                (bounds.top() + bounds.bottom()) / 2.0 / scale as f32 + top as f32;
                            let height =
                                (bounds.bottom() - bounds.top()).max(1.0) / scale as f32;
                            // A tile boundary can split a badge; the overlapping
                            // tile sees it whole, so keep the taller reading.
                            let duplicate = out.iter().position(|existing| {
                                (existing.centre_y - centre_y).abs() < height * 0.5
                                    && (existing.left - bounds.left() / scale as f32 - x0 as f32)
                                        .abs()
                                        < 6.0
                            });
                            let block = TextBlock {
                                left: bounds.left() / scale as f32 + x0 as f32,
                                right: bounds.right() / scale as f32 + x0 as f32,
                                centre_y,
                                height,
                                text: label,
                            };
                            match duplicate {
                                Some(index) => {
                                    if block.height > out[index].height {
                                        out[index] = block;
                                    }
                                }
                                None => out.push(block),
                            }
                        }
        }
        if height < TILE {
            break;
        }
        top += TILE - OVERLAP;
    }
    out
}

/// True when a block is a plausible printed page number.
fn is_number_block(block: &TextBlock) -> bool {
    block.height <= 80.0
        && block.text.chars().any(|c| c.is_ascii_digit())
        && block
            .text
            .chars()
            .all(|c| c.is_ascii_digit() || c.is_whitespace() || c == '.')
}

/// Finds columns of small numeric blocks and re-reads them as vertical strips.
fn augment_number_columns(
    image: &image::RgbImage,
    engine: &ocrs::OcrEngine,
    blocks: &mut Vec<TextBlock>,
) {
    let mut numbers: Vec<TextBlock> = blocks
        .iter()
        .filter(|block| is_number_block(block))
        .cloned()
        .collect();
    numbers.sort_by(|a, b| a.left.partial_cmp(&b.left).unwrap_or(std::cmp::Ordering::Equal));

    // Group numeric blocks into vertical columns by a horizontal gap.
    let mut columns: Vec<Vec<TextBlock>> = Vec::new();
    for block in numbers {
        match columns.last_mut() {
            Some(column)
                if block.left - column.last().map(|b| b.right).unwrap_or(0.0) < 250.0 =>
            {
                column.push(block);
            }
            _ => columns.push(vec![block]),
        }
    }

    for column in columns {
        if column.len() < 2 {
            continue;
        }
        let left = column.iter().fold(f32::INFINITY, |acc, b| acc.min(b.left));
        let right = column.iter().fold(0.0f32, |acc, b| acc.max(b.right));
        let pad = 30.0;
        let x0 = (left - pad).max(0.0) as u32;
        let x1 = (right + pad) as u32;
        for found in ocr_strip(image, engine, x0, x1) {
            if !is_number_block(&found) {
                continue;
            }
            let duplicate = blocks.iter().any(|existing| {
                (existing.centre_y - found.centre_y).abs() < 15.0
                    && (existing.left - found.left).abs() < 40.0
            });
            if !duplicate {
                blocks.push(found);
            }
        }
    }
}

fn ocr_blocks(
    image_bytes: &[u8],
    models: Option<&OcrModels>,
) -> AppResult<Vec<TextBlock>> {
    let models = models
        .ok_or_else(|| AppError::message("OCR models are unavailable in this build"))?;
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
    let recognized = engine
        .recognize_text(&input, &lines)
        .map_err(|err| AppError::message(format!("OCR recognition failed: {err}")))?;

    let mut blocks = Vec::new();
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
        blocks.push(TextBlock {
            left: bounds.left(),
            right: bounds.right(),
            centre_y: (bounds.top() + bounds.bottom()) / 2.0,
            height: (bounds.bottom() - bounds.top()).max(1.0),
            text: label,
        });
    }

    // Page numbers live in small badges and are the part OCR is worst at, so
    // re-read just those blocks magnified. Everything else keeps the fast
    // full-page pass.
    let engine = engine;
    for block in blocks.iter_mut() {
        let looks_numeric = block
            .text
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '.')
            .all(|c| c.is_ascii_digit());
        if !looks_numeric || block.height > 80.0 {
            continue;
        }
        if let Some(value) = reread_number(&image, block, &engine) {
            block.text = value.to_string();
        }
    }
    augment_number_columns(&image, &engine, &mut blocks);
    Ok(blocks)
}

/// Joins blocks whose vertical centres overlap into reading rows.
///
/// A contents page often has two columns, so this merges both columns into one
/// line. Callers that need column identity use [`parse_contents_blocks`]
/// instead; this exists for single-column text and for the line-oriented tests.
fn group_rows(blocks: Vec<TextBlock>) -> Vec<String> {
    let mut blocks = blocks;
    blocks.sort_by(|a, b| {
        a.centre_y
            .partial_cmp(&b.centre_y)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut rows: Vec<(f32, f32, Vec<TextBlock>)> = Vec::new();
    for block in blocks {
        match rows.iter_mut().find(|(centre, height, _)| {
            (centre - block.centre_y).abs() < height * 0.6
        }) {
            Some((_, _, items)) => items.push(block),
            None => rows.push((block.centre_y, block.height, vec![block])),
        }
    }
    rows.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    rows.into_iter()
        .map(|(_, _, mut items)| {
            items.sort_by(|a, b| a.left.partial_cmp(&b.left).unwrap_or(std::cmp::Ordering::Equal));
            items
                .into_iter()
                .map(|block| block.text)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

/// OCR a single page image into text lines.
fn ocr_lines(image_bytes: &[u8], models: Option<&OcrModels>) -> AppResult<Vec<String>> {
    Ok(group_rows(ocr_blocks(image_bytes, models)?))
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

/// Probe helper: OCR a page with a fresh engine, for diagnostics examples.
#[doc(hidden)]
pub fn ocr_lines_for_probe(
    image_bytes: &[u8],
    models: &OcrModels,
) -> AppResult<Vec<String>> {
    ocr_lines(image_bytes, Some(models))
}

/// Probe helper: exposes the contents heuristic to diagnostics examples.
#[doc(hidden)]
pub fn looks_like_contents_probe(lines: &[String]) -> bool {
    looks_like_contents(lines)
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

/// Splits a "SERIES #12" heading into its parts.
///
/// Collected editions label rows with the issue they reprint rather than a
/// chapter number, so this shape is the common one on a trade's contents page.
fn split_series_number(text: &str) -> Option<(String, String)> {
    let hash = text.find('#')?;
    let (series, tail) = text.split_at(hash);
    let number: String = tail[1..]
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    if number.is_empty() {
        return None;
    }
    let series = normalize_title(series).trim_end_matches(':').trim().to_string();
    Some((series, number))
}

/// Parses one OCR line into a chapter row.
///
/// Accepted shapes (OCR-tolerant):
///   "Chapter 1  Fallout   9"
///   "Chapter One - Fallout  9"
///   "Sonic the Hedgehog #21 .... 45"
///   "1. Fallout  9"
///
/// A bare "SERIES #12" heading is accepted only by [`parse_heading`]: on a
/// two-column contents page the page number sits in a separate badge, so the
/// line has no number to read.
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
        || line.contains('#')
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
    let (series, number) = match split_series_number(&title) {
        Some((series, number)) => (Some(series), number),
        None => (None, String::new()),
    };
    Some(ParsedEntry {
        title,
        series,
        number,
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

/// Recognises a standalone "SERIES #12" heading (no page number on the line).
fn parse_heading(raw: &str) -> Option<(String, String, String)> {
    let title = normalize_title(raw);
    if title.len() < 3 {
        return None;
    }
    let (series, number) = split_series_number(&title)?;
    if series.is_empty() {
        return None;
    }
    Some((title, series, number))
}

/// True when a block is nothing but a plausible printed page number.
fn as_page_number(text: &str) -> Option<i64> {
    let trimmed = text.trim().trim_end_matches(['.', ' ']);
    if trimmed.is_empty() || trimmed.len() > 4 {
        return None;
    }
    if !trimmed.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let value = trimmed.parse::<i64>().ok()?;
    (1..=5000).contains(&value).then_some(value)
}

/// Extracts chapter rows from a two-column contents page using positions.
///
/// Trade contents pages put the title block on the left and the page number in
/// a badge to its right, with the number on a different baseline from the
/// title. Grouping by row alone merges the two columns and loses the pairing,
/// so each heading is matched to the nearest unused number to its right, within
/// the vertical band before the next heading.
///
/// Rows are returned in printed-page order. That is the book's reading order
/// regardless of which column the layout puts first, and it makes the result
/// immune to OCR emitting the right column before the left.
pub fn parse_contents_blocks(blocks: &[TextBlock]) -> Vec<ParsedEntry> {
    let mut headings: Vec<&TextBlock> = blocks
        .iter()
        .filter(|block| parse_heading(&block.text).is_some())
        .collect();
    headings.sort_by(|a, b| {
        a.centre_y
            .partial_cmp(&b.centre_y)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.left.partial_cmp(&b.left).unwrap_or(std::cmp::Ordering::Equal))
    });
    if headings.is_empty() {
        return Vec::new();
    }

    // Two-column trade dress puts each page number in a badge between the
    // columns, so the number is to the *right* of a left-column heading and to
    // the *left* of a right-column one. Splitting on the gutter avoids pairing a
    // heading with the badge that belongs to the column beside it.
    let min_left = blocks.iter().fold(f32::INFINITY, |acc, b| acc.min(b.left));
    let max_right = blocks.iter().fold(0.0f32, |acc, b| acc.max(b.right));
    let gutter = (min_left + max_right) / 2.0;

    let numbers: Vec<(i64, &TextBlock)> = blocks
        .iter()
        .filter_map(|block| as_page_number(&block.text).map(|value| (value, block)))
        .collect();
    let mut used = vec![false; numbers.len()];

    let mut entries = Vec::new();
    for heading in &headings {
        let Some((title, series, number)) = parse_heading(&heading.text) else {
            continue;
        };
        let left_column = heading.left < gutter;
        // The badge sits a couple of heading-heights below its title, so the
        // window runs from the title's baseline down to just past that offset.
        let band_top = heading.centre_y - heading.height;
        let band_bottom = heading.centre_y + heading.height * 4.5;

        let mut best: Option<(f32, usize)> = None;
        for (slot, (_, block)) in numbers.iter().enumerate() {
            if used[slot] || block.centre_y < band_top || block.centre_y > band_bottom {
                continue;
            }
            let on_gutter_side = if left_column {
                block.left >= heading.left
            } else {
                block.right <= heading.right
            };
            if !on_gutter_side {
                continue;
            }
            let dx = if left_column {
                (block.left - heading.right).max(0.0)
            } else {
                (heading.left - block.right).max(0.0)
            };
            let dy = (block.centre_y - heading.centre_y).abs();
            let score = dy + dx * 0.1;
            if best.is_none_or(|(current, _)| score < current) {
                best = Some((score, slot));
            }
        }

        let Some((_, slot)) = best else { continue };
        used[slot] = true;
        entries.push(ParsedEntry {
            title,
            series: Some(series),
            number,
            printed_page: numbers[slot].0,
        });
    }

    entries.sort_by_key(|entry| entry.printed_page);
    // A contents page lists each chapter once; a page number seen twice means
    // OCR duplicated a heading, and keeping both would create a zero-length
    // range.
    entries.dedup_by_key(|entry| entry.printed_page);
    entries
}

/// Extracts chapter rows from OCRed lines, in printed order.
pub fn parse_contents(lines: &[String]) -> Vec<ParsedEntry> {
    let mut entries: Vec<ParsedEntry> = lines.iter().filter_map(|line| parse_row(line)).collect();
    entries.sort_by_key(|entry| entry.printed_page);
    entries
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

        // Positioned parsing first: trade contents pages put each page number in
        // a badge away from its title, which line-oriented parsing cannot pair.
        let mut entries = ocr_blocks(&bytes, models)
            .map(|blocks| parse_contents_blocks(&blocks))
            .unwrap_or_default();
        // Fall back to plain lines for single-column "Chapter N ... page" lists.
        if entries.len() < 2
            && let Ok(lines) = ocr_lines(&bytes, models)
            && looks_like_contents(&lines)
        {
            entries = parse_contents(&lines);
        }

        if entries.len() >= 2 {
            let better = best
                .as_ref()
                .is_none_or(|(_, current)| entries.len() > current.len());
            if better {
                best = Some((index, entries));
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
            series: entry.series,
            number: entry.number,
            printed_page: entry.printed_page,
        }
    }
}
