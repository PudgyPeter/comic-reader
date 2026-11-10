use std::io::Read;

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::error::AppResult;

#[derive(Debug, Clone, Default)]
pub struct ComicInfo {
    pub title: Option<String>,
    pub series: Option<String>,
    pub number: Option<String>,
    pub volume: Option<String>,
    pub publisher: Option<String>,
    pub release_date: Option<String>,
    pub writer: Option<String>,
    pub artist: Option<String>,
    pub cover_artist: Option<String>,
    pub description: Option<String>,
    /// Comma-separated `<Characters>` entries, cleaned and de-duplicated.
    pub characters: Vec<String>,
}

const MAX_XML_BYTES: u64 = 4 * 1024 * 1024;

pub fn read_comic_info(path: &std::path::Path) -> AppResult<Option<ComicInfo>> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut archive = match zip::ZipArchive::new(reader) {
        Ok(archive) => archive,
        Err(_) => return Ok(None),
    };
    let mut name = None;
    for index in 0..archive.len() {
        let entry = match archive.by_index(index) {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let Ok(entry_name) = entry.name().map(|name| name.to_string()) else {
            continue;
        };
        let lower = entry_name.to_ascii_lowercase();
        if lower.ends_with("comicinfo.xml") {
            name = Some(entry_name);
            break;
        }
    }
    let Some(name) = name else {
        return Ok(None);
    };
    let mut entry = archive.by_name(&name)?;
    if entry.size() > MAX_XML_BYTES {
        return Ok(None);
    }
    let mut xml = String::new();
    entry.by_ref().take(MAX_XML_BYTES).read_to_string(&mut xml)?;
    Ok(Some(parse_comic_info(&xml)))
}

pub fn parse_comic_info(xml: &str) -> ComicInfo {
    let mut info = ComicInfo::default();
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut current: Option<String> = None;
    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => {
                let name = event.name();
                let local = name.local_name().as_ref().to_ascii_lowercase();
                current = Some(local);
            }
            Ok(Event::End(_)) => current = None,
            Ok(Event::Text(text)) => {
                if let Some(field) = &current {
                    let value = quick_xml::escape::unescape(&text)
                        .map(|value| value.trim().to_string())
                        .unwrap_or_else(|_| text.trim().to_string());
                    if value.is_empty() {
                        continue;
                    }
                    assign(&mut info, field, value);
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    info
}

fn assign(info: &mut ComicInfo, field: &str, value: String) {
    match field {
        "title" => info.title = Some(value),
        "series" | "seriesname" => info.series = Some(value),
        "number" | "issue" => info.number = Some(value),
        "volume" => info.volume = Some(value),
        "publisher" => info.publisher = Some(value),
        "year" => info.release_date = Some(value),
        "month" | "day" => {}
        "writer" => info.writer = Some(value),
        "penciller" | "artist" => {
            if info.artist.is_none() {
                info.artist = Some(value)
            }
        }
        "coverartist" => info.cover_artist = Some(value),
        "summary" | "description" => info.description = Some(value),
        "characters" | "character" => {
            for name in parse_character_list(&value) {
                if !info.characters.iter().any(|known| known.eq_ignore_ascii_case(&name)) {
                    info.characters.push(name);
                }
            }
        }
        _ => {}
    }
}

#[derive(Debug, Clone, Default)]
pub struct ParsedName {
    pub series: String,
    pub number: String,
    pub year: Option<String>,
}

pub fn parse_filename(file_name: &str) -> ParsedName {
    let stem = file_name
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(file_name);
    let cleaned = stem.replace(['_', '.'], " ");
    let cleaned = cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    let tokens: Vec<&str> = cleaned.split(' ').collect();
    let mut series_tokens: Vec<String> = Vec::new();
    let mut number = String::new();
    let mut year = None;

    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index];
        let lower = token.to_ascii_lowercase();
        if number.is_empty() {
            if let Some(rest) = lower.strip_prefix('#')
                && rest.chars().all(|c| c.is_ascii_digit()) && !rest.is_empty() {
                    number = rest.to_string();
                    index += 1;
                    continue;
                }
            if (lower == "issue" || lower == "chapter" || lower == "vol") && index + 1 < tokens.len()
            {
                let next = tokens[index + 1]
                    .trim_start_matches('#')
                    .trim_matches(|c: char| c == ')' || c == '(');
                if next.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                    if lower == "vol" {
                        index += 2;
                        continue;
                    }
                    number = next.to_string();
                    index += 2;
                    continue;
                }
            }
        }
        if let Some((left, _right)) = token.split_once('(')
            && left.trim().is_empty() {
                let digits: String = token.chars().filter(|c| c.is_ascii_digit()).collect();
                if digits.len() == 4 {
                    year = Some(digits);
                }
                index += 1;
                continue;
            }
        if token.starts_with('(') && token.contains(')') {
            let digits: String = token.chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.len() == 4 {
                year = Some(digits);
            }
            index += 1;
            continue;
        }
        series_tokens.push(token.to_string());
        index += 1;
    }

    if number.is_empty() {
        for token in tokens.iter().rev() {
            let digits: String = token
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>();
            if !digits.is_empty() && digits.len() <= 4 {
                number = digits;
                break;
            }
        }
    }

    // A trailing bare number that became the issue number is part of the
    // filename pattern, not the series name ("Sonic 001.cbz" -> "Sonic").
    if !number.is_empty() {
        let normalized = normalize_number(&number);
        while let Some(last) = series_tokens.last() {
            let digits_only = last.chars().all(|c| c.is_ascii_digit());
            if digits_only && normalize_number(last) == normalized {
                series_tokens.pop();
            } else {
                break;
            }
        }
    }

    let series = if series_tokens.is_empty() {
        stem.to_string()
    } else {
        series_tokens.join(" ")
    };

    ParsedName {
        series: series.trim().to_string(),
        number: normalize_number(&number),
        year,
    }
}

fn normalize_number(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.chars().all(|c| c.is_ascii_digit()) && !trimmed.is_empty() {
        let stripped = trimmed.trim_start_matches('0');
        return if stripped.is_empty() {
            "0".to_string()
        } else {
            stripped.to_string()
        };
    }
    trimmed.to_string()
}

pub fn parse_number(value: &str) -> f64 {
    let trimmed = value.trim();
    if let Ok(number) = trimmed.parse::<f64>() {
        return number;
    }
    let mut numeric = String::new();
    let mut suffix = String::new();
    for ch in trimmed.chars() {
        if ch.is_ascii_digit() || (ch == '.' && !numeric.contains('.')) {
            if suffix.is_empty() {
                numeric.push(ch);
            }
        } else if ch.is_ascii_alphabetic() {
            suffix.push(ch);
        }
    }
    let base = numeric.parse::<f64>().unwrap_or(0.0);
    if suffix.is_empty() {
        base
    } else {
        base + (suffix.chars().next().unwrap() as u32 as f64) / 1000.0
    }
}

/// Splits a ComicInfo `<Characters>` value into individual names.
///
/// The tag is free text in the wild: comma, semicolon, slash and "&" are all
/// used as separators, and some editors pack a whole roster into one value.
/// Parenthesised qualifiers ("Sonic (Classic)") are kept because they often
/// distinguish genuinely different characters.
pub fn parse_character_list(raw: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let normalized = raw.replace(" and ", "&");
    for part in normalized.split([',', ';', '/', '&', '\n']) {
        let name = clean_character_name(part);
        if name.is_empty() {
            continue;
        }
        if !out.iter().any(|known| known.eq_ignore_ascii_case(&name)) {
            out.push(name);
        }
    }
    out
}

/// Normalises OCR/comic-info noise out of a single character name.
pub fn clean_character_name(raw: &str) -> String {
    let trimmed = raw
        .trim()
        .trim_matches(|c: char| c == '*' || c == '-' || c == '·' || c == ':');
    let mut cleaned = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");
    // A trailing "and" survives splitting on commas in "Tails, Knuckles and Amy".
    if let Some(stripped) = cleaned.strip_suffix(" and") {
        cleaned = stripped.trim_end().to_string();
    }
    if let Some(stripped) = cleaned.strip_prefix("and ") {
        cleaned = stripped.trim_start().to_string();
    }
    // Drop role qualifiers that some taggers append: "Sonic (protagonist)".
    if cleaned.ends_with(')')
        && let Some((head, _)) = cleaned.rsplit_once('(')
        && !head.trim().is_empty()
        && cleaned.split('(').count() == 2
    {
        cleaned = head.trim_end().to_string();
    }
    if cleaned.chars().count() > 80 {
        return String::new();
    }
    cleaned
}

/// Lowercased sort key: strips leading articles and non-alphanumerics.
pub fn character_sort_name(name: &str) -> String {
    let lower = name.to_lowercase();
    let stripped = ["the ", "a ", "an "]
        .iter()
        .find_map(|prefix| lower.strip_prefix(prefix))
        .unwrap_or(&lower);
    let mut out = String::new();
    for ch in stripped.chars() {
        if ch.is_alphanumeric() {
            out.push(ch);
        } else if ch == ' ' || ch == '-' {
            out.push(' ');
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
