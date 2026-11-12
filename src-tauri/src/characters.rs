//! Series-scoped character roster built from `ComicInfo.xml` `<Characters>`.
//!
//! This is deliberately smaller than the universe database described in
//! `DATA_MODEL.md` §10: one row per character *per series*, an appearance row per
//! issue, and a description with an explicit source. It needs no providers, no
//! OCR and no panel detection, so it works fully offline and can be derived from
//! files the user already imported.
//!
//! Descriptions are never invented silently. `compose_description` produces a
//! short factual summary from the appearances the library actually contains; a
//! web summary is only written when the user asks for it, and it is tagged
//! `description_source = 'web'` with the URL it came from.

use rusqlite::{params, Connection, OptionalExtension};

use crate::db::{new_id, now};
use crate::error::{AppError, AppResult};
use crate::metadata;
use crate::models::{CharacterAppearance, CharacterCard, CharacterDetail, CharacterPatch};

/// Outcome of a roster scan, shown after the user asks to rebuild it.
#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterSync {
    pub series_id: String,
    pub characters: i64,
    pub appearances: i64,
    pub issues_scanned: i64,
    pub issues_without_metadata: i64,
}

/// Records the `<Characters>` tag of one freshly imported issue.
///
/// Called inside the import transaction, so a failure here rolls the import
/// back rather than leaving a partially indexed roster.
pub fn record_issue_characters(
    conn: &Connection,
    series_id: &str,
    issue_id: &str,
    names: &[String],
) -> AppResult<()> {
    if names.is_empty() {
        return Ok(());
    }
    let timestamp = now();
    for name in names {
        let cleaned = metadata::clean_character_name(name);
        if cleaned.is_empty() {
            continue;
        }
        let character_id = ensure_character(conn, series_id, &cleaned, &timestamp)?;
        conn.execute(
            "INSERT OR IGNORE INTO character_appearances
                (id, character_id, issue_id, source, created_at)
             VALUES (?1, ?2, ?3, 'comicinfo', ?4)",
            params![new_id(), character_id, issue_id, timestamp],
        )?;
    }
    Ok(())
}

/// Finds a character by name within the series, creating it on first sight.
///
/// Names are matched case-insensitively so a re-import after a metadata edit
/// does not fork "sonic" and "Sonic" into two rows.
pub fn ensure_character(
    conn: &Connection,
    series_id: &str,
    name: &str,
    timestamp: &str,
) -> AppResult<String> {
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM characters WHERE series_id = ?1 AND lower(name) = lower(?2)",
            params![series_id, name],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        return Ok(id);
    }
    let id = new_id();
    conn.execute(
        "INSERT INTO characters
            (id, series_id, name, sort_name, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![
            id,
            series_id,
            name,
            metadata::character_sort_name(name),
            timestamp
        ],
    )?;
    Ok(id)
}

/// Rebuilds a series roster from the ComicInfo of every issue that has a file.
///
/// Idempotent: appearances are `INSERT OR IGNORE`, and existing characters keep
/// their descriptions (local, manual or web) untouched.
pub fn sync_series(conn: &Connection, series_id: &str) -> AppResult<CharacterSync> {
    let mut report = CharacterSync {
        series_id: series_id.to_string(),
        ..Default::default()
    };
    let mut stmt = conn.prepare(
        "SELECT i.id, f.path FROM issues i
         JOIN library_files f ON f.id = i.file_id
         WHERE i.series_id = ?1
         ORDER BY i.sort_number",
    )?;
    let rows = stmt.query_map([series_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let issues = rows.collect::<Result<Vec<_>, _>>()?;
    for (issue_id, path) in issues {
        report.issues_scanned += 1;
        let info = metadata::read_comic_info(std::path::Path::new(&path))
            .ok()
            .flatten();
        let Some(info) = info else {
            report.issues_without_metadata += 1;
            continue;
        };
        if info.characters.is_empty() {
            report.issues_without_metadata += 1;
            continue;
        }
        record_issue_characters(conn, series_id, &issue_id, &info.characters)?;
    }
    report.characters = conn.query_row(
        "SELECT count(*) FROM characters WHERE series_id = ?1",
        [series_id],
        |row| row.get(0),
    )?;
    report.appearances = conn.query_row(
        "SELECT count(*) FROM character_appearances a
         JOIN characters c ON c.id = a.character_id
         WHERE c.series_id = ?1",
        [series_id],
        |row| row.get(0),
    )?;
    Ok(report)
}

/// Characters for a series, alphabetical, with their appearance counts.
pub fn series_characters(conn: &Connection, series_id: &str) -> AppResult<Vec<CharacterCard>> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.series_id, s.title, c.name, c.description, c.description_source,
                c.source_url,
                (SELECT count(*) FROM character_appearances a JOIN issues i ON i.id = a.issue_id
                 WHERE a.character_id = c.id),
                (SELECT count(*) FROM character_appearances a JOIN issues i ON i.id = a.issue_id
                 JOIN progress p ON p.issue_id = i.id
                 WHERE a.character_id = c.id AND p.status = 'completed'),
                (SELECT count(*) FROM character_relationships r
                 WHERE r.character_a_id = c.id OR r.character_b_id = c.id),
                c.description_updated_at
         FROM characters c LEFT JOIN series s ON s.id = c.series_id
         WHERE c.series_id = ?1
         ORDER BY c.sort_name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([series_id], |row| {
        Ok(CharacterCard {
            id: row.get(0)?,
            series_id: row.get(1)?,
            series_title: row.get(2)?,
            name: row.get(3)?,
            description: row.get(4)?,
            description_source: row.get(5)?,
            source_url: row.get(6)?,
            appearance_count: row.get(7)?,
            read_count: row.get(8)?,
            relationship_count: row.get(9)?,
            description_updated_at: row.get(10)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// One character with its appearances, newest issue last.
///
/// Appearances are spoiler-sensitive: an issue beyond the reader's frontier is
/// reported with `spoiler_protected` and no title rather than being dropped, so
/// the count stays honest without naming the issue.
pub fn character_detail(conn: &Connection, character_id: &str) -> AppResult<CharacterDetail> {
    let card: Option<CharacterCard> = conn
        .query_row(
            "SELECT c.id, c.series_id, s.title, c.name, c.description, c.description_source,
                    c.source_url,
                    (SELECT count(*) FROM character_appearances a WHERE a.character_id = c.id),
                    (SELECT count(*) FROM character_appearances a JOIN issues i ON i.id = a.issue_id
                     JOIN progress p ON p.issue_id = i.id
                     WHERE a.character_id = c.id AND p.status = 'completed'),
                    (SELECT count(*) FROM character_relationships r
                     WHERE r.character_a_id = c.id OR r.character_b_id = c.id),
                    c.description_updated_at
             FROM characters c LEFT JOIN series s ON s.id = c.series_id
             WHERE c.id = ?1",
            [character_id],
            |row| {
                Ok(CharacterCard {
                    id: row.get(0)?,
                    series_id: row.get(1)?,
                    series_title: row.get(2)?,
                    name: row.get(3)?,
                    description: row.get(4)?,
                    description_source: row.get(5)?,
                    source_url: row.get(6)?,
                    appearance_count: row.get(7)?,
                    read_count: row.get(8)?,
                    relationship_count: row.get(9)?,
                    description_updated_at: row.get(10)?,
                })
            },
        )
        .optional()?;
    let Some(card) = card else {
        return Err(AppError::not_found("character"));
    };
    let config = crate::repo::settings(conn)?;
    let series_id: Option<String> = conn.query_row(
        "SELECT series_id FROM characters WHERE id = ?1",
        [character_id],
        |row| row.get(0),
    )?;
    let mut stmt = conn.prepare(
        "SELECT i.id, i.title, i.number, i.series_id, i.sort_number, i.release_date,
                coalesce(p.status, 'unread'), i.description
         FROM character_appearances a
         JOIN issues i ON i.id = a.issue_id
         LEFT JOIN progress p ON p.issue_id = i.id
         WHERE a.character_id = ?1
         ORDER BY i.sort_number, i.number",
    )?;
    let rows = stmt.query_map([character_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, f64>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, Option<String>>(7)?,
        ))
    })?;
    let mut appearances = Vec::new();
    let mut probes: Vec<crate::models::IssueCard> = Vec::new();
    let mut meta = Vec::new();
    for row in rows {
        let (issue_id, title, number, issue_series, sort_number, release_date, status, _) = row?;
        // Build a minimal card so the shared spoiler filter sees exactly the
        // same input it would for a library or series query.
        probes.push(crate::models::IssueCard {
            id: issue_id.clone(),
            series_id: issue_series.clone().or_else(|| series_id.clone()),
            series_title: None,
            title: title.clone(),
            number: number.clone(),
            sort_number,
            volume: None,
            publisher: None,
            universe: None,
            release_date: release_date.clone(),
            writer: None,
            artist: None,
            cover_artist: None,
            description: None,
            file_id: None,
            file_path: None,
            page_count: 0,
            last_page: 0,
            status: status.clone(),
            last_read_at: None,
            completed_at: None,
            spoiler_protected: false,
            missing: false,
            variant_of: None,
            edition: None,
            variant_count: 0,
            metadata_locked: false,
            page_offset: 0,
        });
        meta.push((issue_id, title, number, issue_series, sort_number, release_date, status));
    }
    crate::repo::apply_spoilers(conn, &config, &mut probes)?;
    for (index, (issue_id, title, number, issue_series, sort_number, release_date, status)) in
        meta.into_iter().enumerate()
    {
        let protected = probes[index].spoiler_protected;
        appearances.push(CharacterAppearance {
            issue_id,
            title: if protected { String::new() } else { title },
            number,
            series_id: issue_series.or_else(|| series_id.clone()),
            sort_number,
            release_date,
            status,
            spoiler_protected: protected,
        });
    }
    Ok(CharacterDetail {
        card,
        appearances,
        relationships: crate::relationships::for_character(conn, character_id)?,
    })
}

/// Stores a user-written or web-derived description, tagging its provenance.
pub fn update_character(
    conn: &Connection,
    character_id: &str,
    patch: &CharacterPatch,
) -> AppResult<CharacterCard> {
    let exists: Option<String> = conn
        .query_row("SELECT id FROM characters WHERE id = ?1", [character_id], |row| {
            row.get(0)
        })
        .optional()?;
    if exists.is_none() {
        return Err(AppError::not_found("character"));
    }
    if let Some(description) = patch.description.as_ref() {
        let source = patch
            .description_source
            .as_deref()
            .filter(|value| matches!(*value, "comic" | "web" | "manual"))
            .unwrap_or("manual");
        conn.execute(
            "UPDATE characters SET description = ?2, description_source = ?3,
                source_url = ?4, description_updated_at = ?5, updated_at = ?5
             WHERE id = ?1",
            params![
                character_id,
                description.trim(),
                source,
                patch.source_url.as_deref().filter(|url| !url.trim().is_empty()),
                now()
            ],
        )?;
    }
    if let Some(name) = patch.name.as_ref().filter(|name| !name.trim().is_empty()) {
        conn.execute(
            "UPDATE characters SET name = ?2, sort_name = ?3, updated_at = ?4 WHERE id = ?1",
            params![
                character_id,
                name.trim(),
                metadata::character_sort_name(name),
                now()
            ],
        )?;
    }
    if let Some(aliases) = patch.aliases.as_ref() {
        conn.execute(
            "DELETE FROM character_aliases WHERE character_id = ?1",
            [character_id],
        )?;
        for alias in aliases {
            let cleaned = alias.trim();
            if cleaned.is_empty() {
                continue;
            }
            conn.execute(
                "INSERT OR IGNORE INTO character_aliases (id, character_id, alias)
                 VALUES (?1, ?2, ?3)",
                params![new_id(), character_id, cleaned],
            )?;
        }
    }
    let detail = character_detail(conn, character_id)?;
    Ok(detail.card)
}

pub fn delete_character(conn: &Connection, character_id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM characters WHERE id = ?1", [character_id])?;
    Ok(())
}

/// Builds a short description from what the library actually knows.
///
/// Deliberately factual: it lists the series, the number of known appearances,
/// the first issue it appears in, and the aliases on record. It never infers
/// personality or plot, because a wrong sentence about a character is worse than
/// a plain one.
pub fn compose_description(conn: &Connection, character_id: &str) -> AppResult<String> {
    let (name, series_id): (String, Option<String>) = conn
        .query_row(
            "SELECT name, series_id FROM characters WHERE id = ?1",
            [character_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("character"))?;
    let series_title: Option<String> = match series_id.as_ref() {
        Some(id) => conn
            .query_row("SELECT title FROM series WHERE id = ?1", [id], |row| row.get(0))
            .optional()?,
        None => None,
    };
    let mut stmt = conn.prepare(
        "SELECT coalesce(i.number, ''), i.title FROM character_appearances a
         JOIN issues i ON i.id = a.issue_id
         WHERE a.character_id = ?1 ORDER BY i.sort_number, i.number",
    )?;
    let rows = stmt.query_map([character_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let appearances = rows.collect::<Result<Vec<_>, _>>()?;
    if appearances.is_empty() {
        return Ok(String::new());
    }
    let mut parts = Vec::new();
    let first = &appearances[0];
    if let Some(series) = series_title.as_ref() {
        parts.push(format!(
            "{name} appears in {series}, first in {}#{}.",
            series,
            first.0
        ));
    } else {
        parts.push(format!("{name} first appears in #{}", first.0));
    }
    parts.push(format!(
        "The library records {} known {}.",
        appearances.len(),
        if appearances.len() == 1 {
            "appearance"
        } else {
            "appearances"
        }
    ));
    let mut alias_stmt = conn.prepare(
        "SELECT alias FROM character_aliases WHERE character_id = ?1 ORDER BY alias COLLATE NOCASE",
    )?;
    let aliases = alias_stmt
        .query_map([character_id], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    if !aliases.is_empty() {
        parts.push(format!("Also known as {}.", aliases.join(", ")));
    }
    Ok(parts.join(" "))
}

/// Writes the composed local description when the character has none yet.
pub fn ensure_local_description(conn: &Connection, character_id: &str) -> AppResult<()> {
    let current: Option<(Option<String>, String)> = conn
        .query_row(
            "SELECT description, description_source FROM characters WHERE id = ?1",
            [character_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((description, source)) = current else {
        return Err(AppError::not_found("character"));
    };
    if description.as_deref().is_some_and(|text| !text.trim().is_empty()) {
        return Ok(());
    }
    let composed = compose_description(conn, character_id)?;
    if composed.is_empty() {
        return Ok(());
    }
    conn.execute(
        "UPDATE characters SET description = ?2, description_source = 'comic',
            source_url = NULL, description_updated_at = ?3, updated_at = ?3
         WHERE id = ?1 AND (description IS NULL OR trim(description) = '')",
        params![character_id, composed, now()],
    )?;
    let _ = source;
    Ok(())
}

/// Every character in the library, alphabetical, for the global roster page.
pub fn all_characters(conn: &Connection) -> AppResult<Vec<CharacterCard>> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.series_id, s.title, c.name, c.description, c.description_source,
                c.source_url,
                (SELECT count(*) FROM character_appearances a WHERE a.character_id = c.id),
                (SELECT count(*) FROM character_appearances a JOIN issues i ON i.id = a.issue_id
                 JOIN progress p ON p.issue_id = i.id
                 WHERE a.character_id = c.id AND p.status = 'completed'),
                (SELECT count(*) FROM character_relationships r
                 WHERE r.character_a_id = c.id OR r.character_b_id = c.id),
                c.description_updated_at
         FROM characters c LEFT JOIN series s ON s.id = c.series_id
         ORDER BY c.sort_name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(CharacterCard {
            id: row.get(0)?,
            series_id: row.get(1)?,
            series_title: row.get(2)?,
            name: row.get(3)?,
            description: row.get(4)?,
            description_source: row.get(5)?,
            source_url: row.get(6)?,
            appearance_count: row.get(7)?,
            read_count: row.get(8)?,
            relationship_count: row.get(9)?,
            description_updated_at: row.get(10)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Number of distinct characters known for a series, for empty-state copy.
pub fn series_character_count(conn: &Connection, series_id: &str) -> AppResult<i64> {
    Ok(conn.query_row(
        "SELECT count(*) FROM characters WHERE series_id = ?1",
        [series_id],
        |row| row.get(0),
    )?)
}
