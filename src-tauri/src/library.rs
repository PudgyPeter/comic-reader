use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};
use walkdir::WalkDir;

use crate::archive;
use crate::db::{new_id, now};
use crate::error::{AppError, AppResult};
use crate::metadata;
use crate::models::*;

const MAX_SCAN_FILES: usize = 50_000;

pub fn is_archive_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".cbz")
        || lower.ends_with(".zip")
        || lower.ends_with(".cbr")
        || lower.ends_with(".pdf")
}

fn folder_has_images(path: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(path) else {
        return false;
    };
    let mut images = 0usize;
    for entry in entries.flatten() {
        if !entry.file_type().map(|kind| kind.is_file()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if archive::is_image_path(&name) {
            images += 1;
            if images >= 2 {
                return true;
            }
        }
    }
    false
}

pub fn normalize_root(path: &Path) -> AppResult<PathBuf> {
    // `dunce` strips the Windows verbatim `\\?\` prefix that `canonicalize`
    // adds, so paths stay readable and comparable with values produced by the
    // shell, the folder picker, and existing database rows.
    let canonical = dunce::canonicalize(path)
        .map_err(|_| AppError::invalid(format!("cannot open folder: {}", path.display())))?;
    if !canonical.is_dir() {
        return Err(AppError::invalid("library root must be a folder"));
    }
    Ok(canonical)
}

/// Paths are stored with forward slashes so a library stays comparable across
/// platforms and across Windows versions. Reading back converts again.
pub fn to_stored_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn paths_equal(a: &str, b: &str) -> bool {
    let a = a.replace('\\', "/");
    let b = b.replace('\\', "/");
    if cfg!(windows) {
        a.eq_ignore_ascii_case(&b)
    } else {
        a == b
    }
}

pub fn read_roots(conn: &Connection) -> AppResult<Vec<LibraryRoot>> {
    let mut stmt = conn.prepare(
        "SELECT r.id, r.path, r.added_at,
                (SELECT COUNT(*) FROM library_files f WHERE f.root_id = r.id)
         FROM library_roots r ORDER BY r.added_at",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(LibraryRoot {
            id: row.get(0)?,
            path: row.get(1)?,
            added_at: row.get(2)?,
            file_count: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn ensure_root(conn: &Connection, path: &Path) -> AppResult<(String, bool)> {
    let now_value = now();
    let stored = to_stored_path(path);
    let mut stmt = conn.prepare("SELECT id, path FROM library_roots")?;
    let existing = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    if let Some((id, _)) = existing
        .into_iter()
        .find(|(_, existing_path)| paths_equal(existing_path, &stored))
    {
        return Ok((id, false));
    }
    let id = new_id();
    conn.execute(
        "INSERT INTO library_roots (id, path, added_at) VALUES (?1, ?2, ?3)",
        params![id, stored, now_value],
    )?;
    Ok((id, true))
}

pub fn remove_root(conn: &Connection, root_id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM library_roots WHERE id = ?1", [root_id])?;
    Ok(())
}

/// Cooperative cancellation + progress reporting for a library scan.
#[derive(Default)]
pub struct ImportControl {
    cancel: std::sync::atomic::AtomicBool,
}

impl ImportControl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancel.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn reset(&self) {
        self.cancel.store(false, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProgress {
    pub stage: String,
    pub processed: i64,
    pub total: i64,
    pub current: String,
}

pub fn import_root(conn: &mut Connection, path: &Path) -> AppResult<ImportResult> {
    import_root_with_progress(conn, path, &ImportControl::new(), |_| {})
}

pub fn import_root_with_progress(
    conn: &mut Connection,
    path: &Path,
    control: &ImportControl,
    mut report: impl FnMut(ImportProgress),
) -> AppResult<ImportResult> {
    let root = normalize_root(path)?;
    let (root_id, _) = ensure_root(conn, &root)?;

    let mut candidates: Vec<PathBuf> = Vec::new();
    let mut warnings: Vec<ImportWarning> = Vec::new();
    let walker = WalkDir::new(&root).max_depth(6).follow_links(false);
    for entry in walker.into_iter().filter_map(|entry| entry.ok()) {
        if candidates.len() >= MAX_SCAN_FILES {
            warnings.push(ImportWarning {
                path: root.to_string_lossy().to_string(),
                message: "Scan stopped after 50,000 files".into(),
                kind: "limit".into(),
            });
            break;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name.starts_with('$') {
            continue;
        }
        if entry.file_type().is_file() {
            if is_archive_name(&name) {
                candidates.push(entry.path().to_path_buf());
            }
            continue;
        }
        if entry.file_type().is_dir() && entry.path() != root.as_path() {
            // A folder that directly contains images is treated as one comic,
            // matching the "image folder" archive format.
            if folder_has_images(entry.path()) {
                candidates.push(entry.path().to_path_buf());
            }
        }
    }
    candidates.sort();
    if control.is_cancelled() {
        return Err(AppError::cancelled());
    }

    let total = candidates.len() as i64;
    report(ImportProgress {
        stage: "importing".into(),
        processed: 0,
        total,
        current: String::new(),
    });

    let mut files_imported = 0i64;
    let mut files_skipped = 0i64;
    let mut series_created = 0i64;
    let mut issues_created = 0i64;

    let existing: HashSet<String> = {
        let mut stmt = conn.prepare("SELECT relative_path FROM library_files WHERE root_id = ?1")?;
        let rows = stmt.query_map([&root_id], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<HashSet<_>, _>>()?
    };

    for (index, candidate) in candidates.into_iter().enumerate() {
        if control.is_cancelled() {
            report(ImportProgress {
                stage: "cancelled".into(),
                processed: index as i64,
                total,
                current: to_stored_path(&candidate),
            });
            return Err(AppError::cancelled());
        }
        report(ImportProgress {
            stage: "importing".into(),
            processed: index as i64,
            total,
            current: to_stored_path(&candidate),
        });
        let relative = candidate
            .strip_prefix(&root)
            .unwrap_or(&candidate)
            .to_string_lossy()
            .to_string();
        if existing.iter().any(|stored| paths_equal(stored, &relative)) {
            files_skipped += 1;
            continue;
        }
        match import_file(conn, &root_id, &candidate, &relative) {
            Ok(Some(outcome)) => {
                files_imported += 1;
                if outcome.series_created {
                    series_created += 1;
                }
                if outcome.issue_created {
                    issues_created += 1;
                }
                warnings.extend(outcome.warnings);
            }
            Ok(None) => files_skipped += 1,
            Err(err) => warnings.push(ImportWarning {
                path: to_stored_path(&candidate),
                message: err.to_string(),
                kind: "archive".into(),
            }),
        }
    }

    report(ImportProgress {
        stage: "done".into(),
        processed: total,
        total,
        current: String::new(),
    });

    let root_record = read_roots(conn)?
        .into_iter()
        .find(|root| root.id == root_id)
        .ok_or_else(|| AppError::not_found("library root"))?;

    Ok(ImportResult {
        root: root_record,
        files_imported,
        files_skipped,
        series_created,
        issues_created,
        warnings,
    })
}

struct FileOutcome {
    series_created: bool,
    issue_created: bool,
    warnings: Vec<ImportWarning>,
}

fn import_file(
    conn: &mut Connection,
    root_id: &str,
    path: &Path,
    relative: &str,
) -> AppResult<Option<FileOutcome>> {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    let lower = name.to_ascii_lowercase();
    let format = if path.is_dir() {
        "folder"
    } else if lower.ends_with(".cbz") || lower.ends_with(".zip") {
        "cbz"
    } else if lower.ends_with(".cbr") {
        "cbr"
    } else if lower.ends_with(".pdf") {
        "pdf"
    } else {
        return Ok(None);
    };

    let pages = match format {
        "folder" => archive::read_folder(path)?,
        "cbr" => archive::read_rar(path)?,
        "pdf" => archive::read_pdf_entries(path)?,
        _ => archive::read_archive(path)?,
    };
    if pages.is_empty() {
        return Ok(None);
    }

    let size = std::fs::metadata(path)?.len() as i64;
    let comic_info = metadata::read_comic_info(path).ok().flatten();
    let parsed = metadata::parse_filename(&name);

    let tx = conn.transaction()?;
    let file_id = new_id();
    let timestamp = now();
    tx.execute(
        "INSERT INTO library_files
            (id, root_id, path, relative_path, format, size_bytes, page_count, status, added_at, scanned_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'present', ?8, ?8)",
        params![
            file_id,
            root_id,
            to_stored_path(path),
            relative.replace('\\', "/"),
            format,
            size,
            pages.len() as i64,
            timestamp
        ],
    )?;

    let mut warnings = Vec::new();
    for (index, page) in pages.iter().enumerate() {
        tx.execute(
            "INSERT INTO file_pages (file_id, page_index, entry_name, width, height)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                file_id,
                index as i64,
                page.name,
                page.width as i64,
                page.height as i64
            ],
        )?;
    }

    let series_title = comic_info
        .as_ref()
        .and_then(|info| info.series.clone())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            if parsed.series.trim().is_empty() {
                name.clone()
            } else {
                parsed.series.clone()
            }
        });
    let number = comic_info
        .as_ref()
        .and_then(|info| info.number.clone())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| parsed.number.clone());
    let title = comic_info
        .as_ref()
        .and_then(|info| info.title.clone())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            if number.is_empty() {
                series_title.clone()
            } else {
                format!("{series_title} #{number}")
            }
        });
    let publisher = comic_info.as_ref().and_then(|info| info.publisher.clone());
    let release_date = comic_info
        .as_ref()
        .and_then(|info| info.release_date.clone())
        .or(parsed.year.clone());

    let (series_id, series_created) = ensure_series(&tx, &series_title, publisher.as_deref())?;
    let sort_number = metadata::parse_number(&number);

    // Warn on likely duplicates without blocking the import. A duplicate is the
    // same series + number already present, or an identical source fingerprint.
    let duplicate: Option<String> = tx
        .query_row(
            "SELECT i.title FROM issues i
             WHERE i.series_id = ?1 AND i.number = ?2
             LIMIT 1",
            params![series_id, number],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(existing_title) = duplicate {
        warnings.push(ImportWarning {
            path: path.to_string_lossy().to_string(),
            message: format!("Possible duplicate: “{existing_title}” already covers this number"),
            kind: "duplicate".into(),
        });
    }

    tx.execute(
        "INSERT INTO issues
            (id, file_id, series_id, title, number, sort_number, volume, publisher, universe,
             release_date, writer, artist, cover_artist, description, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, ?10, ?11, ?12, ?13, ?14, ?14)",
        params![
            new_id(),
            file_id,
            series_id,
            title,
            number,
            sort_number,
            comic_info.as_ref().and_then(|info| info.volume.clone()),
            publisher,
            release_date,
            comic_info.as_ref().and_then(|info| info.writer.clone()),
            comic_info.as_ref().and_then(|info| info.artist.clone()),
            comic_info.as_ref().and_then(|info| info.cover_artist.clone()),
            comic_info.as_ref().and_then(|info| info.description.clone()),
            timestamp
        ],
    )?;
    tx.execute(
        "INSERT INTO progress (issue_id, page, total, status, updated_at)
         SELECT id, 0, ?1, 'unread', ?2 FROM issues WHERE file_id = ?3",
        params![pages.len() as i64, timestamp, file_id],
    )?;
    tx.commit()?;

    if let Some(cover) = pages.first() {
        let _ = cover;
    }

    Ok(Some(FileOutcome {
        series_created,
        issue_created: true,
        warnings: {
            if pages.len() < 2 {
                warnings.push(ImportWarning {
                    path: to_stored_path(path),
                    message: "Archive contains a single page".into(),
                    kind: "pages".into(),
                });
            }
            warnings
        },
    }))
}

pub fn ensure_series(
    conn: &Connection,
    title: &str,
    publisher: Option<&str>,
) -> AppResult<(String, bool)> {
    let trimmed = title.trim();
    let existing = conn
        .query_row(
            "SELECT id FROM series WHERE lower(title) = lower(?1)
             AND coalesce(publisher, '') = coalesce(?2, '') AND coalesce(universe, '') = ''",
            params![trimmed, publisher],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    if let Some(id) = existing {
        return Ok((id, false));
    }
    let id = new_id();
    let timestamp = now();
    conn.execute(
        "INSERT INTO series (id, title, sort_title, publisher, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![id, trimmed, trimmed.to_lowercase(), publisher, timestamp],
    )?;
    Ok((id, true))
}

pub fn rescan(conn: &mut Connection) -> AppResult<Vec<ImportResult>> {
    rescan_with_progress(conn, &ImportControl::new(), |_| {})
}

pub fn rescan_with_progress(
    conn: &mut Connection,
    control: &ImportControl,
    mut report: impl FnMut(ImportProgress),
) -> AppResult<Vec<ImportResult>> {
    let roots = read_roots(conn)?;
    let mut results = Vec::new();
    for root in roots {
        if control.is_cancelled() {
            return Err(AppError::cancelled());
        }
        let path = PathBuf::from(&root.path);
        if !path.exists() {
            conn.execute(
                "UPDATE library_files SET status = 'missing' WHERE root_id = ?1",
                [&root.id],
            )?;
            continue;
        }
        results.push(import_root_with_progress(conn, &path, control, &mut report)?);
        conn.execute(
            "UPDATE library_files SET status = 'present' WHERE root_id = ?1 AND status = 'missing'",
            [&root.id],
        )?;
    }
    Ok(results)
}

pub const DISPLAY_FIELDS: &str =
    "i.id, i.series_id, s.title, i.title, i.number, i.sort_number, i.volume, i.publisher, \
     i.universe, i.release_date, i.writer, i.artist, i.cover_artist, i.description, i.file_id, \
     f.path, f.page_count, coalesce(p.page, 0), coalesce(p.status, 'unread'), p.last_read_at, \
     p.completed_at, coalesce(f.status, 'missing'), i.variant_of, i.edition, \
     (SELECT count(*) FROM issues v WHERE v.variant_of = i.id)";

pub fn map_issue(row: &rusqlite::Row<'_>) -> rusqlite::Result<IssueCard> {
    map_issue_offset(row, 0)
}

pub fn map_issue_offset(row: &rusqlite::Row<'_>, base: usize) -> rusqlite::Result<IssueCard> {
    let file_status: String = row.get(base + 21)?;
    Ok(IssueCard {
        id: row.get(base)?,
        series_id: row.get(base + 1)?,
        series_title: row.get(base + 2)?,
        title: row.get(base + 3)?,
        number: row.get(base + 4)?,
        sort_number: row.get(base + 5)?,
        volume: row.get(base + 6)?,
        publisher: row.get(base + 7)?,
        universe: row.get(base + 8)?,
        release_date: row.get(base + 9)?,
        writer: row.get(base + 10)?,
        artist: row.get(base + 11)?,
        cover_artist: row.get(base + 12)?,
        description: row.get(base + 13)?,
        file_id: row.get(base + 14)?,
        file_path: row.get(base + 15)?,
        page_count: row.get(base + 16)?,
        last_page: row.get(base + 17)?,
        status: row.get(base + 18)?,
        last_read_at: row.get(base + 19)?,
        completed_at: row.get(base + 20)?,
        spoiler_protected: false,
        missing: file_status != "present",
        variant_of: row.get(base + 22)?,
        edition: row.get(base + 23)?,
        variant_count: row.get(base + 24)?,
    })
}
