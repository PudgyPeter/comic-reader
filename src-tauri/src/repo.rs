use std::collections::{BTreeMap, HashMap, HashSet};

use rusqlite::{params, params_from_iter, Connection, OptionalExtension, ToSql};

use crate::archive;
use crate::db::{new_id, now};
use crate::error::{AppError, AppResult};
use crate::library::{self, DISPLAY_FIELDS};
use crate::metadata;
use crate::models::*;

pub fn issue_select() -> String {
    format!("SELECT {DISPLAY_FIELDS} FROM issues i \
             LEFT JOIN series s ON s.id = i.series_id \
             LEFT JOIN library_files f ON f.id = i.file_id \
             LEFT JOIN progress p ON p.issue_id = i.id")
}

pub fn settings(conn: &Connection) -> AppResult<Settings> {
    let mut settings = Settings::default();
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (key, value) = row?;
        match key.as_str() {
            "background" => settings.background = value,
            "pageSpacing" => {
                settings.page_spacing = value.parse().unwrap_or(settings.page_spacing)
            }
            "readingDirection" => settings.reading_direction = value,
            "defaultMode" => settings.default_mode = value,
            "defaultFit" => settings.default_fit = value,
            "defaultZoomBehavior" => settings.default_zoom_behavior = value,
            "transition" => settings.transition = value,
            "transitionDuration" => {
                settings.transition_duration = value.parse().unwrap_or(settings.transition_duration)
            }
            "animationsEnabled" => settings.animations_enabled = value == "true",
            "showInterface" => settings.show_interface = value == "true",
            "fullscreen" => settings.fullscreen = value == "true",
            "completionThreshold" => {
                settings.completion_threshold = value.parse().unwrap_or(settings.completion_threshold)
            }
            "spoilerProtection" => settings.spoiler_protection = value == "true",
            "spoilerOrderId" => {
                settings.spoiler_order_id = if value.is_empty() { None } else { Some(value) }
            }
            "theme" => settings.theme = value,
            "accent" => settings.accent = value,
            "density" => settings.density = value,
            "libraryLayout" => settings.library_layout = value,
            "librarySort" => settings.library_sort = value,
            "shortcuts" => {
                if let Ok(map) =
                    serde_json::from_str::<BTreeMap<String, String>>(&value)
                {
                    settings.shortcuts = map;
                }
            }
            _ => {}
        }
    }
    let mut shortcuts = Settings::default().shortcuts;
    shortcuts.extend(settings.shortcuts.clone());
    settings.shortcuts = shortcuts;
    Ok(settings)
}

pub fn save_settings(conn: &mut Connection, incoming: &Settings) -> AppResult<Settings> {
    let tx = conn.transaction()?;
    let pairs: Vec<(&str, String)> = vec![
        ("background", incoming.background.clone()),
        ("pageSpacing", incoming.page_spacing.to_string()),
        ("readingDirection", incoming.reading_direction.clone()),
        ("defaultMode", incoming.default_mode.clone()),
        ("defaultFit", incoming.default_fit.clone()),
        ("defaultZoomBehavior", incoming.default_zoom_behavior.clone()),
        ("transition", incoming.transition.clone()),
        ("transitionDuration", incoming.transition_duration.to_string()),
        ("animationsEnabled", incoming.animations_enabled.to_string()),
        ("showInterface", incoming.show_interface.to_string()),
        ("fullscreen", incoming.fullscreen.to_string()),
        ("completionThreshold", incoming.completion_threshold.to_string()),
        ("spoilerProtection", incoming.spoiler_protection.to_string()),
        (
            "spoilerOrderId",
            incoming.spoiler_order_id.clone().unwrap_or_default(),
        ),
        ("theme", incoming.theme.clone()),
        ("accent", incoming.accent.clone()),
        ("density", incoming.density.clone()),
        ("libraryLayout", incoming.library_layout.clone()),
        ("librarySort", incoming.library_sort.clone()),
        (
            "shortcuts",
            serde_json::to_string(&incoming.shortcuts).unwrap_or_else(|_| "{}".into()),
        ),
    ];
    for (key, value) in pairs {
        tx.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
    }
    tx.commit()?;
    settings(conn)
}

fn frontier_release(conn: &Connection) -> AppResult<Option<String>> {
    conn.query_row(
        "SELECT max(i.release_date) FROM issues i
         JOIN progress p ON p.issue_id = i.id
         WHERE p.status = 'completed' AND i.release_date IS NOT NULL",
        [],
        |row| row.get::<_, Option<String>>(0),
    )
    .map_err(Into::into)
}

fn order_frontier(conn: &Connection, order_id: &str) -> AppResult<Option<f64>> {
    conn.query_row(
        "SELECT max(r.position) FROM reading_order_items r
         JOIN progress p ON p.issue_id = r.issue_id
         WHERE r.order_id = ?1 AND p.status = 'completed'",
        [order_id],
        |row| row.get::<_, Option<f64>>(0),
    )
    .map_err(Into::into)
}

fn order_positions(conn: &Connection, order_id: &str) -> AppResult<HashMap<String, f64>> {
    let mut stmt = conn.prepare(
        "SELECT issue_id, position FROM reading_order_items WHERE order_id = ?1",
    )?;
    let rows = stmt.query_map([order_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    })?;
    Ok(rows.collect::<Result<HashMap<_, _>, _>>()?)
}

fn revealed_issues(conn: &Connection) -> AppResult<HashSet<String>> {
    let mut stmt = conn.prepare("SELECT issue_id FROM spoiler_reveals")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<Result<HashSet<_>, _>>()?)
}

fn series_frontiers(conn: &Connection) -> AppResult<HashMap<String, f64>> {
    let mut stmt = conn.prepare(
        "SELECT i.series_id, max(i.sort_number) FROM issues i
         JOIN progress p ON p.issue_id = i.id
         WHERE p.status = 'completed' AND i.series_id IS NOT NULL
         GROUP BY i.series_id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    })?;
    Ok(rows.collect::<Result<HashMap<_, _>, _>>()?)
}

fn apply_spoilers(
    conn: &Connection,
    settings: &Settings,
    issues: &mut [IssueCard],
) -> AppResult<()> {
    if !settings.spoiler_protection {
        return Ok(());
    }
    let frontier = frontier_release(conn)?;
    let revealed = revealed_issues(conn)?;
    let series_frontier = series_frontiers(conn)?;

    // Spec: spoiler protection follows the user's selected reading order when one
    // is configured, and falls back to per-series progress otherwise.
    let order = settings
        .spoiler_order_id
        .as_ref()
        .filter(|id| !id.is_empty());
    let (order_frontier, order_positions_map) = match order {
        Some(order_id) => (
            order_frontier(conn, order_id)?,
            order_positions(conn, order_id)?,
        ),
        None => (None, HashMap::new()),
    };

    let issue_frontier: Option<f64> = conn
        .query_row(
            "SELECT max(i.sort_number) FROM issues i
             JOIN progress p ON p.issue_id = i.id
             WHERE p.status = 'completed'",
            [],
            |row| row.get::<_, Option<f64>>(0),
        )
        .optional()?
        .flatten();

    for issue in issues.iter_mut() {
        if issue.status == "completed" || revealed.contains(&issue.id) {
            continue;
        }
        if let Some(order_id) = order {
            let in_order = order_positions_map.get(&issue.id).copied();
            match (in_order, order_frontier) {
                (Some(position), Some(max)) => {
                    if position <= max {
                        continue;
                    }
                    issue.description = None;
                    issue.spoiler_protected = true;
                    continue;
                }
                (Some(_), None) => {
                    issue.description = None;
                    issue.spoiler_protected = true;
                    continue;
                }
                (None, _) => {
                    let _ = order_id;
                }
            }
        }
        let safe_by_series = issue
            .series_id
            .as_ref()
            .and_then(|series_id| series_frontier.get(series_id).copied())
            .is_some_and(|max| issue.sort_number <= max);
        let safe_by_date = match (&frontier, &issue.release_date) {
            (Some(frontier), Some(date)) => date.as_str() < frontier.as_str(),
            _ => false,
        };
        let safe_globally = issue_frontier.is_some_and(|max| issue.sort_number <= max)
            && issue.series_id.is_none();
        if safe_by_series || safe_by_date || safe_globally {
            continue;
        }
        issue.description = None;
        issue.spoiler_protected = true;
    }
    Ok(())
}

fn sort_clause(sort: &str, descending: bool) -> String {
    let direction = if descending { "DESC" } else { "ASC" };
    match sort {
        "title" => format!("i.title COLLATE NOCASE {direction}"),
        "series" => format!("s.title COLLATE NOCASE {direction}, i.sort_number ASC"),
        "number" => format!("i.sort_number {direction}, i.number COLLATE NOCASE"),
        "release" => format!(
            "CASE WHEN i.release_date IS NULL THEN 1 ELSE 0 END, i.release_date {direction}"
        ),
        "recent" => format!(
            "CASE WHEN p.last_read_at IS NULL THEN 1 ELSE 0 END, p.last_read_at {direction}"
        ),
        "added" => format!("i.created_at {direction}"),
        _ => format!(
            "CASE WHEN p.last_read_at IS NULL THEN 1 ELSE 0 END, p.last_read_at {direction}"
        ),
    }
}

pub fn library_snapshot(conn: &Connection, filter: &LibraryFilter) -> AppResult<LibrarySnapshot> {
    let config = settings(conn)?;
    let mut conditions: Vec<String> = Vec::new();
    let mut values: Vec<Box<dyn ToSql>> = Vec::new();

    if !filter.search.trim().is_empty() {
        conditions.push(
            "(i.title LIKE ? OR s.title LIKE ? OR i.number LIKE ? OR i.writer LIKE ? \
              OR i.artist LIKE ? OR i.publisher LIKE ? OR i.description LIKE ?)"
                .into(),
        );
        let needle = format!("%{}%", filter.search.trim());
        for _ in 0..7 {
            values.push(Box::new(needle.clone()));
        }
    }
    if let Some(universe) = filter.universe.as_ref().filter(|v| !v.is_empty()) {
        conditions.push("i.universe = ?".into());
        values.push(Box::new(universe.clone()));
    }
    if let Some(publisher) = filter.publisher.as_ref().filter(|v| !v.is_empty()) {
        conditions.push("i.publisher = ?".into());
        values.push(Box::new(publisher.clone()));
    }
    if let Some(series_id) = filter.series_id.as_ref().filter(|v| !v.is_empty()) {
        conditions.push("i.series_id = ?".into());
        values.push(Box::new(series_id.clone()));
    }
    if let Some(status) = filter.status.as_ref().filter(|v| !v.is_empty()) {
        conditions.push("coalesce(p.status, 'unread') = ?".into());
        values.push(Box::new(status.clone()));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };
    let order = sort_clause(&filter.sort, filter.descending);
    let sql = format!("{} {} ORDER BY {}", issue_select(), where_clause, order);
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(values.iter().map(|v| v.as_ref())), library::map_issue)?;
    let mut issues = rows.collect::<Result<Vec<_>, _>>()?;
    apply_spoilers(conn, &config, &mut issues)?;

    let series = series_summaries(conn)?;
    let roots = library::read_roots(conn)?;

    let totals = conn.query_row(
        "SELECT
            (SELECT COUNT(*) FROM issues),
            (SELECT COUNT(*) FROM series),
            (SELECT COUNT(*) FROM progress WHERE status = 'completed'),
            (SELECT COUNT(*) FROM progress WHERE status = 'in-progress'),
            (SELECT COUNT(*) FROM progress WHERE status = 'unread'),
            (SELECT coalesce(sum(page), 0) FROM progress)",
        [],
        |row| {
            Ok(LibraryTotals {
                issues: row.get(0)?,
                series: row.get(1)?,
                completed: row.get(2)?,
                in_progress: row.get(3)?,
                unread: row.get(4)?,
                pages_read: row.get(5)?,
            })
        },
    )?;

    Ok(LibrarySnapshot {
        issues,
        series,
        roots,
        totals,
    })
}

pub fn series_summaries(conn: &Connection) -> AppResult<Vec<SeriesSummary>> {
    let sql = "
        SELECT s.id, s.title, s.publisher, s.universe, s.description,
            (SELECT COUNT(*) FROM issues i WHERE i.series_id = s.id),
            (SELECT COUNT(*) FROM issues i JOIN progress p ON p.issue_id = i.id
             WHERE i.series_id = s.id AND p.status = 'completed'),
            coalesce(s.cover_file_id,
                (SELECT i.file_id FROM issues i WHERE i.series_id = s.id AND i.file_id IS NOT NULL
                 ORDER BY i.sort_number LIMIT 1)),
            s.cover_page, s.year, s.reading_direction
        FROM series s
        ORDER BY s.title COLLATE NOCASE";
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| {
        Ok(SeriesSummary {
            id: row.get(0)?,
            title: row.get(1)?,
            publisher: row.get(2)?,
            universe: row.get(3)?,
            description: row.get(4)?,
            issue_count: row.get(5)?,
            read_count: row.get(6)?,
            cover_file_id: row.get(7)?,
            cover_page: row.get(8)?,
            year: row.get(9)?,
            reading_direction: row.get(10)?,
            missing_count: 0,
        })
    })?;
    let mut series = rows.collect::<Result<Vec<_>, _>>()?;
    fill_missing_counts(conn, &mut series)?;
    Ok(series)
}

pub fn series_detail(conn: &Connection, series_id: &str) -> AppResult<SeriesDetail> {
    let config = settings(conn)?;
    let summary = series_summaries(conn)?
        .into_iter()
        .find(|series| series.id == series_id)
        .ok_or_else(|| AppError::not_found("series"))?;
    let sql = format!(
        "{} WHERE i.series_id = ?1 ORDER BY i.sort_number ASC, i.title COLLATE NOCASE",
        issue_select()
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([series_id], library::map_issue)?;
    let mut issues = rows.collect::<Result<Vec<_>, _>>()?;
    apply_spoilers(conn, &config, &mut issues)?;
    let missing = missing_issues(conn, series_id, &issues)?;
    Ok(SeriesDetail { summary, issues, missing })
}

/// Computes gaps in a series' integer numbering and merges user overrides.
///
/// Only numeric issue numbers participate: specials, annuals and one-shots are
/// visible as issues but never influence the gap range. Overrides live in
/// `missing_issues` and can mark a gap as "not-required" or add numbers the
/// scan cannot infer (for example a range that starts after the first issue).
pub fn missing_issues(
    conn: &Connection,
    series_id: &str,
    issues: &[IssueCard],
) -> AppResult<Vec<MissingIssue>> {
    let numbers = issues
        .iter()
        .filter_map(|issue| issue.number.trim().parse::<i64>().ok())
        .filter(|number| *number > 0)
        .collect::<Vec<_>>();
    let overrides = series_overrides(conn, series_id)?;
    Ok(missing_from(&numbers, &overrides))
}

fn series_overrides(conn: &Connection, series_id: &str) -> AppResult<HashMap<String, String>> {
    let mut overrides: HashMap<String, String> = HashMap::new();
    let mut stmt = conn.prepare(
        "SELECT number, reason FROM missing_issues WHERE series_id = ?1",
    )?;
    let rows = stmt.query_map([series_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (number, reason) = row?;
        overrides.insert(number, reason);
    }
    Ok(overrides)
}

fn missing_from(numbers: &[i64], overrides: &HashMap<String, String>) -> Vec<MissingIssue> {
    let present: HashSet<i64> = numbers.iter().copied().collect();
    let mut found: Vec<MissingIssue> = Vec::new();
    if let (Some(min), Some(max)) = (
        numbers.iter().copied().min(),
        numbers.iter().copied().max(),
    ) {
        // Guard against absurd ranges from a stray issue number like 5000.
        let span = max - min;
        if span <= 500 {
            for number in min..=max {
                if present.contains(&number) {
                    continue;
                }
                let key = number.to_string();
                if overrides.get(&key).map(String::as_str) == Some("not-required") {
                    continue;
                }
                found.push(MissingIssue {
                    number: key,
                    reason: "missing".into(),
                });
            }
        }
    }

    // Overrides can also add explicit entries that are outside the computed range.
    for (number, reason) in overrides {
        if reason == "not-required" || present.contains(&number.parse::<i64>().unwrap_or(-1)) {
            continue;
        }
        if found.iter().any(|entry| entry.number == *number) {
            continue;
        }
        found.push(MissingIssue {
            number: number.clone(),
            reason: reason.clone(),
        });
    }

    found.sort_by(|a, b| {
        let left = a.number.parse::<i64>().ok();
        let right = b.number.parse::<i64>().ok();
        match (left, right) {
            (Some(left), Some(right)) => left.cmp(&right),
            _ => a.number.cmp(&b.number),
        }
    });
    found
}

fn fill_missing_counts(conn: &Connection, series: &mut [SeriesSummary]) -> AppResult<()> {
    let mut numbers: HashMap<String, Vec<i64>> = HashMap::new();
    let mut stmt = conn.prepare("SELECT series_id, number FROM issues WHERE series_id IS NOT NULL")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (series_id, number) = row?;
        if let Ok(value) = number.trim().parse::<i64>()
            && value > 0
        {
            numbers.entry(series_id).or_default().push(value);
        }
    }

    let mut overrides: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut stmt = conn.prepare("SELECT series_id, number, reason FROM missing_issues")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (series_id, number, reason) = row?;
        overrides.entry(series_id).or_default().insert(number, reason);
    }

    for entry in series.iter_mut() {
        let present = numbers.get(&entry.id).map(Vec::as_slice).unwrap_or(&[]);
        let empty = HashMap::new();
        let reasons = overrides.get(&entry.id).unwrap_or(&empty);
        entry.missing_count = missing_from(present, reasons).len() as i64;
    }
    Ok(())
}

pub fn set_missing_reason(
    conn: &Connection,
    series_id: &str,
    number: &str,
    reason: Option<&str>,
) -> AppResult<()> {
    match reason {
        None => {
            conn.execute(
                "DELETE FROM missing_issues WHERE series_id = ?1 AND number = ?2",
                params![series_id, number],
            )?;
        }
        Some(reason) if reason == "missing" || reason == "not-required" => {
            conn.execute(
                "INSERT INTO missing_issues (series_id, number, reason, updated_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(series_id, number) DO UPDATE SET reason = excluded.reason,
                    updated_at = excluded.updated_at",
                params![series_id, number, reason, now()],
            )?;
        }
        Some(_) => {
            return Err(AppError::invalid(
                "missing reason must be missing or not-required",
            ));
        }
    }
    Ok(())
}

pub fn filter_options(conn: &Connection) -> AppResult<serde_json::Value> {
    let collect = |column: &str| -> AppResult<Vec<String>> {
        let sql = format!(
            "SELECT DISTINCT {column} FROM issues WHERE {column} IS NOT NULL AND {column} <> '' ORDER BY {column}"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    };
    let universes = collect("universe")?;
    let publishers = collect("publisher")?;
    Ok(serde_json::json!({ "universes": universes, "publishers": publishers }))
}

pub fn issue(conn: &Connection, issue_id: &str) -> AppResult<IssueCard> {
    let sql = format!("{} WHERE i.id = ?1", issue_select());
    let mut stmt = conn.prepare(&sql)?;
    let mut issue = stmt
        .query_row([issue_id], library::map_issue)
        .optional()?
        .ok_or_else(|| AppError::not_found("issue"))?;
    let config = settings(conn)?;
    apply_spoilers(conn, &config, std::slice::from_mut(&mut issue))?;
    Ok(issue)
}

pub fn reader_payload(conn: &Connection, issue_id: &str) -> AppResult<ReaderPayload> {
    let config = settings(conn)?;
    let card = issue(conn, issue_id)?;
    let file_id = card
        .file_id
        .clone()
        .ok_or_else(|| AppError::not_found("issue has no readable file"))?;
    let (path, format): (String, String) = conn.query_row(
        "SELECT path, format FROM library_files WHERE id = ?1",
        [&file_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let mut stmt = conn.prepare(
        "SELECT page_index, entry_name, width, height FROM file_pages
         WHERE file_id = ?1 ORDER BY page_index",
    )?;
    let rows = stmt.query_map([&file_id], |row| {
        Ok(PageInfo {
            index: row.get(0)?,
            name: row.get(1)?,
            width: row.get(2)?,
            height: row.get(3)?,
        })
    })?;
    let pages = rows.collect::<Result<Vec<_>, _>>()?;
    if pages.is_empty() {
        let _ = format;
        let _ = path;
        return Err(AppError::invalid("no pages found in archive"));
    }

    let progress = conn
        .query_row(
            "SELECT page, total, status, started_at, completed_at, updated_at
             FROM progress WHERE issue_id = ?1",
            [issue_id],
            |row| {
                Ok(ProgressState {
                    page: row.get(0)?,
                    total: row.get(1)?,
                    status: row.get(2)?,
                    started_at: row.get(3)?,
                    completed_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        )
        .optional()?
        .unwrap_or(ProgressState {
            page: 0,
            total: pages.len() as i64,
            status: "unread".into(),
            started_at: None,
            completed_at: None,
            updated_at: None,
        });

    let previous_issue_id = neighbour_issue(conn, &card, false)?;
    let next_issue_id = neighbour_issue(conn, &card, true)?;
    let spread_offset: i64 = conn
        .query_row(
            "SELECT spread_offset FROM file_preferences WHERE file_id = ?1",
            [&file_id],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or(0);
    let file_direction: Option<String> = conn
        .query_row(
            "SELECT reading_direction FROM file_preferences WHERE file_id = ?1",
            [&file_id],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    let series_direction: Option<String> = match card.series_id.as_ref() {
        Some(series_id) => conn
            .query_row(
                "SELECT reading_direction FROM series WHERE id = ?1",
                [series_id],
                |row| row.get(0),
            )
            .optional()?
            .flatten(),
        None => None,
    };
    let reading_direction = file_direction
        .or(series_direction)
        .unwrap_or(config.reading_direction);

    Ok(ReaderPayload {
        file_id,
        issue_id: Some(card.id),
        title: card.title,
        series_title: card.series_title,
        number: card.number,
        pages,
        progress,
        background: config.background,
        mode: config.default_mode,
        reading_direction,
        fit: config.default_fit,
        transition: config.transition,
        transition_duration: config.transition_duration,
        show_interface: config.show_interface,
        next_issue_id,
        previous_issue_id,
        spread_offset,
    })
}

pub fn set_spread_offset(conn: &Connection, issue_id: &str, offset: i64) -> AppResult<i64> {
    let file_id: String = conn.query_row(
        "SELECT file_id FROM issues WHERE id = ?1",
        [issue_id],
        |row| row.get::<_, Option<String>>(0),
    ).optional()?.flatten().ok_or_else(|| AppError::not_found("issue file"))?;
    let offset = offset.rem_euclid(2);
    conn.execute(
        "INSERT INTO file_preferences (file_id, spread_offset, updated_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(file_id) DO UPDATE SET spread_offset = excluded.spread_offset,
            updated_at = excluded.updated_at",
        params![file_id, offset, now()],
    )?;
    Ok(offset)
}

pub fn set_direction_override(
    conn: &Connection,
    issue_id: &str,
    direction: Option<&str>,
) -> AppResult<Option<String>> {
    if let Some(value) = direction
        && value != "ltr"
        && value != "rtl"
    {
        return Err(AppError::invalid("reading direction must be ltr or rtl"));
    }
    let file_id: String = conn
        .query_row(
            "SELECT file_id FROM issues WHERE id = ?1",
            [issue_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten()
        .ok_or_else(|| AppError::not_found("issue file"))?;
    conn.execute(
        "INSERT INTO file_preferences (file_id, spread_offset, reading_direction, updated_at)
         VALUES (?1, 0, ?2, ?3)
         ON CONFLICT(file_id) DO UPDATE SET reading_direction = excluded.reading_direction,
            updated_at = excluded.updated_at",
        params![file_id, direction, now()],
    )?;
    Ok(direction.map(str::to_string))
}

fn neighbour_issue(
    conn: &Connection,
    card: &IssueCard,
    forward: bool,
) -> AppResult<Option<String>> {
    let Some(series_id) = card.series_id.as_ref() else {
        return Ok(None);
    };
    let sql = if forward {
        "SELECT id FROM issues WHERE series_id = ?1 AND sort_number > ?2
         ORDER BY sort_number ASC LIMIT 1"
    } else {
        "SELECT id FROM issues WHERE series_id = ?1 AND sort_number < ?2
         ORDER BY sort_number DESC LIMIT 1"
    };
    conn.query_row(sql, params![series_id, card.sort_number], |row| {
        row.get::<_, String>(0)
    })
    .optional()
    .map_err(Into::into)
}

pub fn save_progress(
    conn: &mut Connection,
    issue_id: &str,
    page: i64,
    completion_threshold: Option<f64>,
) -> AppResult<ProgressState> {
    let config = settings(conn)?;
    let threshold = completion_threshold.unwrap_or(config.completion_threshold);
    let (total, old_status, started_at): (i64, String, Option<String>) = conn
        .query_row(
            "SELECT coalesce(total, 0), coalesce(status, 'unread'), started_at
             FROM progress WHERE issue_id = ?1",
            [issue_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?
        .unwrap_or((0, "unread".into(), None));
    let page = page.max(0);
    let ratio = if total > 0 {
        (page as f64 + 1.0) / total as f64
    } else {
        0.0
    };
    let status = if old_status == "completed" || ratio >= threshold {
        "completed"
    } else if page > 0 || ratio > 0.0 {
        "in-progress"
    } else {
        old_status.as_str()
    };
    let timestamp = now();
    let completed_at = if status == "completed" {
        conn.query_row(
            "SELECT completed_at FROM progress WHERE issue_id = ?1",
            [issue_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten()
        .or(Some(timestamp.clone()))
    } else {
        None
    };
    conn.execute(
        "INSERT INTO progress (issue_id, page, total, status, started_at, completed_at, last_read_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
         ON CONFLICT(issue_id) DO UPDATE SET
            page = excluded.page,
            total = excluded.total,
            status = excluded.status,
            started_at = coalesce(progress.started_at, excluded.started_at),
            completed_at = coalesce(excluded.completed_at, progress.completed_at),
            last_read_at = excluded.last_read_at,
            updated_at = excluded.updated_at",
        params![
            issue_id,
            page,
            total,
            status,
            started_at.unwrap_or_else(|| timestamp.clone()),
            completed_at,
            timestamp
        ],
    )?;
    Ok(ProgressState {
        page,
        total,
        status: status.into(),
        started_at: None,
        completed_at,
        updated_at: Some(timestamp),
    })
}

pub fn set_status(conn: &Connection, issue_id: &str, status: &str) -> AppResult<IssueCard> {
    if !["unread", "in-progress", "completed"].contains(&status) {
        return Err(AppError::invalid("unknown status"));
    }
    let timestamp = now();
    let total: i64 = conn.query_row(
        "SELECT coalesce(total, 0) FROM progress WHERE issue_id = ?1",
        [issue_id],
        |row| row.get(0),
    ).unwrap_or(0);
    let page = if status == "unread" {
        0
    } else if status == "completed" {
        (total - 1).max(0)
    } else {
        0
    };
    let started_at = if status == "unread" { None } else { Some(timestamp.clone()) };
    let completed_at = if status == "completed" { Some(timestamp.clone()) } else { None };
    conn.execute(
        "INSERT INTO progress (issue_id, page, total, status, started_at, completed_at, last_read_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
         ON CONFLICT(issue_id) DO UPDATE SET
            page = excluded.page,
            status = excluded.status,
            started_at = excluded.started_at,
            completed_at = excluded.completed_at,
            last_read_at = excluded.last_read_at,
            updated_at = excluded.updated_at",
        params![issue_id, page, total, status, started_at, completed_at, timestamp],
    )?;
    issue(conn, issue_id)
}

pub fn update_issue(conn: &mut Connection, issue_id: &str, patch: &IssuePatch) -> AppResult<IssueCard> {
    let tx = conn.transaction()?;
    let mut series_id = patch.series_id.clone();
    if let Some(title) = patch.series_title.as_ref().filter(|title| !title.trim().is_empty()) {
        let (id, _) = library::ensure_series(&tx, title, patch.publisher.as_deref())?;
        series_id = Some(id);
    }
    tx.execute(
        "UPDATE issues SET
            title = coalesce(?2, title),
            number = coalesce(?3, number),
            sort_number = coalesce(?4, sort_number),
            volume = coalesce(?5, volume),
            publisher = coalesce(?6, publisher),
            release_date = coalesce(?7, release_date),
            writer = coalesce(?8, writer),
            artist = coalesce(?9, artist),
            cover_artist = coalesce(?10, cover_artist),
            description = coalesce(?11, description),
            series_id = coalesce(?12, series_id),
            variant_of = CASE
                WHEN ?14 IS NULL THEN variant_of
                WHEN ?14 = '' THEN NULL
                ELSE ?14
            END,
            edition = CASE
                WHEN ?15 IS NULL THEN edition
                WHEN ?15 = '' THEN NULL
                ELSE ?15
            END,
            updated_at = ?13
         WHERE id = ?1",
        params![
            issue_id,
            patch.title,
            patch.number,
            patch.number.as_ref().map(|number| metadata::parse_number(number)),
            patch.volume,
            patch.publisher,
            patch.release_date,
            patch.writer,
            patch.artist,
            patch.cover_artist,
            patch.description,
            series_id,
            now(),
            patch.variant_of,
            patch.edition
        ],
    )?;
    if patch.variant_of.is_some() {
        let variant_of = patch.variant_of.as_deref();
        let current: Option<String> = tx.query_row(
            "SELECT variant_of FROM issues WHERE id = ?1",
            [issue_id],
            |row| row.get(0),
        )?;
        if current.as_deref() == Some(issue_id) {
            tx.execute(
                "UPDATE issues SET variant_of = NULL WHERE id = ?1",
                [issue_id],
            )?;
        } else if let Some(parent) = variant_of {
            let same: Option<String> = tx.query_row(
                "SELECT id FROM issues WHERE id = ?1 AND variant_of = ?2",
                params![parent, issue_id],
                |row| row.get(0),
            ).optional()?;
            if same.is_some() {
                return Err(AppError::invalid("variant links cannot form a cycle"));
            }
        }
    }
    if let Some(series) = series_id.as_ref()
        && (patch.publisher.is_some() || patch.release_date.is_some()) {
            let publisher: Option<String> = tx.query_row(
                "SELECT publisher FROM issues WHERE id = ?1",
                [issue_id],
                |row| row.get(0),
            )?;
            tx.execute(
                "UPDATE issues SET publisher = coalesce(publisher, ?2) WHERE series_id = ?1",
                params![series, publisher],
            )?;
        }
    tx.commit()?;
    issue(conn, issue_id)
}

pub fn bulk_update(conn: &mut Connection, issue_ids: &[String], patch: &IssuePatch) -> AppResult<Vec<IssueCard>> {
    let mut out = Vec::with_capacity(issue_ids.len());
    for id in issue_ids {
        out.push(update_issue(conn, id, patch)?);
    }
    Ok(out)
}

pub fn update_series(conn: &Connection, series_id: &str, patch: &SeriesPatch) -> AppResult<SeriesDetail> {
    conn.execute(
        "UPDATE series SET
            title = coalesce(?2, title),
            sort_title = lower(coalesce(?2, title)),
            publisher = coalesce(?3, publisher),
            universe = coalesce(?4, universe),
            description = coalesce(?5, description),
            volume = coalesce(?6, volume),
            cover_file_id = coalesce(?7, cover_file_id),
            reading_direction = CASE
                WHEN ?9 IS NULL THEN reading_direction
                WHEN ?9 = '' THEN NULL
                ELSE ?9
            END,
            updated_at = ?8
         WHERE id = ?1",
        params![
            series_id,
            patch.title,
            patch.publisher,
            patch.universe,
            patch.description,
            patch.volume,
            patch.cover_file_id,
            now(),
            patch.reading_direction
        ],
    )?;
    let universe: Option<String> = conn.query_row(
        "SELECT universe FROM series WHERE id = ?1",
        [series_id],
        |row| row.get(0),
    )?;
    if let Some(universe) = universe {
        conn.execute(
            "UPDATE issues SET universe = ?2 WHERE series_id = ?1",
            params![series_id, universe],
        )?;
    }
    series_detail(conn, series_id)
}

pub fn create_series(conn: &Connection, title: &str, publisher: Option<&str>, universe: Option<&str>) -> AppResult<SeriesDetail> {
    if title.trim().is_empty() {
        return Err(AppError::invalid("series title is required"));
    }
    let (id, created) = library::ensure_series(conn, title, publisher)?;
    if !created
        && let Some(universe) = universe {
            conn.execute(
                "UPDATE series SET universe = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, universe, now()],
            )?;
        }
    series_detail(conn, &id)
}

pub fn delete_series(conn: &Connection, series_id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM series WHERE id = ?1", [series_id])?;
    Ok(())
}

pub fn delete_issue(conn: &Connection, issue_id: &str) -> AppResult<()> {
    let file_id: Option<String> = conn
        .query_row("SELECT file_id FROM issues WHERE id = ?1", [issue_id], |row| row.get(0))
        .optional()?
        .flatten();
    conn.execute("DELETE FROM issues WHERE id = ?1", [issue_id])?;
    if let Some(file_id) = file_id {
        conn.execute("DELETE FROM library_files WHERE id = ?1", [file_id])?;
    }
    Ok(())
}

pub fn search(conn: &Connection, query: &str, limit: i64) -> AppResult<SearchResult> {
    let config = settings(conn)?;
    let needle = format!("%{}%", query.trim());
    let sql = format!(
        "{} WHERE i.title LIKE ?1 OR s.title LIKE ?1 OR i.number LIKE ?1 OR i.writer LIKE ?1 \
         OR i.artist LIKE ?1 OR i.publisher LIKE ?1 OR i.release_date LIKE ?1 OR i.description LIKE ?1 \
         OR EXISTS (SELECT 1 FROM bookmarks b WHERE b.issue_id = i.id
                    AND (b.note LIKE ?1 OR b.label LIKE ?1)) \
         ORDER BY s.title COLLATE NOCASE, i.sort_number LIMIT ?2",
        issue_select()
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![needle, limit], library::map_issue)?;
    let mut issues = rows.collect::<Result<Vec<_>, _>>()?;
    apply_spoilers(conn, &config, &mut issues)?;
    let series_needle = format!("%{}%", query.trim());
    let mut stmt = conn.prepare(
        "SELECT s.id, s.title, s.publisher, s.universe, s.description,
            (SELECT COUNT(*) FROM issues i WHERE i.series_id = s.id),
            (SELECT COUNT(*) FROM issues i JOIN progress p ON p.issue_id = i.id
             WHERE i.series_id = s.id AND p.status = 'completed'),
            coalesce(s.cover_file_id,
                (SELECT i.file_id FROM issues i WHERE i.series_id = s.id AND i.file_id IS NOT NULL
                 ORDER BY i.sort_number LIMIT 1)),
            s.cover_page, s.year, s.reading_direction
         FROM series s WHERE s.title LIKE ?1 ORDER BY s.title COLLATE NOCASE LIMIT ?2",
    )?;
    let series_rows = stmt.query_map(params![series_needle, limit], |row| {
        Ok(SeriesSummary {
            id: row.get(0)?,
            title: row.get(1)?,
            publisher: row.get(2)?,
            universe: row.get(3)?,
            description: row.get(4)?,
            issue_count: row.get(5)?,
            read_count: row.get(6)?,
            cover_file_id: row.get(7)?,
            cover_page: row.get(8)?,
            year: row.get(9)?,
            reading_direction: row.get(10)?,
            missing_count: 0,
        })
    })?;
    let mut series = series_rows.collect::<Result<Vec<_>, _>>()?;
    fill_missing_counts(conn, &mut series)?;
    let total = issues.len() as i64 + series.len() as i64;
    Ok(SearchResult { issues, series, total })
}

pub fn suggestions(conn: &Connection, query: &str) -> AppResult<Vec<SearchSuggestion>> {
    let needle = format!("{}%", query.trim());
    let mut out = Vec::new();
    for (kind, sql) in [
        ("series", "SELECT DISTINCT title FROM series WHERE title LIKE ?1 LIMIT 5"),
        ("writer", "SELECT DISTINCT writer FROM issues WHERE writer LIKE ?1 LIMIT 3"),
        ("publisher", "SELECT DISTINCT publisher FROM issues WHERE publisher LIKE ?1 LIMIT 3"),
    ] {
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([&needle], |row| row.get::<_, String>(0))?;
        for row in rows {
            let value = row?;
            if !value.trim().is_empty() {
                out.push(SearchSuggestion { value, kind: kind.into() });
            }
        }
    }
    Ok(out)
}

pub fn list_saved_searches(conn: &Connection) -> AppResult<Vec<SavedSearch>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, query, created_at FROM saved_searches ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(SavedSearch {
            id: row.get(0)?,
            name: row.get(1)?,
            query: row.get(2)?,
            created_at: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn save_search(conn: &Connection, name: &str, query: &str) -> AppResult<SavedSearch> {
    let id = new_id();
    let timestamp = now();
    conn.execute(
        "INSERT INTO saved_searches (id, name, query, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![id, name, query, timestamp],
    )?;
    Ok(SavedSearch {
        id,
        name: name.into(),
        query: query.into(),
        created_at: timestamp,
    })
}

pub fn delete_saved_search(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM saved_searches WHERE id = ?1", [id])?;
    Ok(())
}

pub fn orders(conn: &Connection) -> AppResult<Vec<ReadingOrder>> {
    let mut stmt = conn.prepare(
        "SELECT o.id, o.name, o.description, o.kind,
            (SELECT COUNT(*) FROM reading_order_items r WHERE r.order_id = o.id),
            o.created_at
         FROM reading_orders o ORDER BY o.created_at",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ReadingOrder {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            kind: row.get(3)?,
            item_count: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn order_detail(conn: &Connection, order_id: &str) -> AppResult<ReadingOrderDetail> {
    let config = settings(conn)?;
    let order = orders(conn)?
        .into_iter()
        .find(|order| order.id == order_id)
        .ok_or_else(|| AppError::not_found("reading order"))?;
    let sql = format!(
        "SELECT r.issue_id, r.position, {DISPLAY_FIELDS}
         FROM reading_order_items r
         JOIN issues i ON i.id = r.issue_id
         LEFT JOIN series s ON s.id = i.series_id
         LEFT JOIN library_files f ON f.id = i.file_id
         LEFT JOIN progress p ON p.issue_id = i.id
         WHERE r.order_id = ?1 ORDER BY r.position"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([order_id], |row| {
        let issue_id: String = row.get(0)?;
        let position: f64 = row.get(1)?;
        let issue = library::map_issue_offset(row, 2)?;
        Ok(OrderItem {
            issue_id,
            position,
            issue: Some(issue),
        })
    })?;
    let mut items = rows.collect::<Result<Vec<_>, _>>()?;
    let mut cards: Vec<IssueCard> = items.iter().filter_map(|item| item.issue.clone()).collect();
    apply_spoilers(conn, &config, &mut cards)?;
    let mut cards = cards.into_iter();
    for item in items.iter_mut() {
        if item.issue.is_some() {
            item.issue = cards.next();
        }
    }
    Ok(ReadingOrderDetail { order, items })
}

pub fn create_order(conn: &Connection, name: &str, description: Option<&str>, kind: &str) -> AppResult<ReadingOrder> {
    let id = new_id();
    let timestamp = now();
    conn.execute(
        "INSERT INTO reading_orders (id, name, description, kind, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![id, name, description, kind, timestamp],
    )?;
    Ok(ReadingOrder {
        id,
        name: name.into(),
        description: description.map(Into::into),
        kind: kind.into(),
        item_count: 0,
        created_at: timestamp,
    })
}

pub fn build_publication_order(conn: &mut Connection, name: &str) -> AppResult<ReadingOrderDetail> {
    let order = create_order(conn, name, Some("Generated from release dates"), "publication")?;
    let tx = conn.transaction()?;
    // Alternate covers are the same story as their base issue, so a generated
    // publication order must not list the variant as an extra read.
    let mut stmt = tx.prepare(
        "SELECT id FROM issues
         WHERE variant_of IS NULL
         ORDER BY CASE WHEN release_date IS NULL THEN 1 ELSE 0 END, release_date,
                  CASE WHEN series_id IS NULL THEN 1 ELSE 0 END, series_id, sort_number",
    )?;
    let ids = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    for (index, id) in ids.iter().enumerate() {
        tx.execute(
            "INSERT INTO reading_order_items (id, order_id, issue_id, position) VALUES (?1, ?2, ?3, ?4)",
            params![new_id(), order.id, id, index as f64],
        )?;
    }
    tx.commit()?;
    order_detail(conn, &order.id)
}

pub fn delete_order(conn: &Connection, order_id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM reading_orders WHERE id = ?1", [order_id])?;
    Ok(())
}

pub fn add_order_items(conn: &Connection, order_id: &str, issue_ids: &[String]) -> AppResult<()> {
    let max: f64 = conn
        .query_row(
            "SELECT coalesce(max(position), -1) FROM reading_order_items WHERE order_id = ?1",
            [order_id],
            |row| row.get(0),
        )
        .unwrap_or(-1.0);
    let mut position = max;
    for issue_id in issue_ids {
        position += 1.0;
        conn.execute(
            "INSERT OR IGNORE INTO reading_order_items (id, order_id, issue_id, position)
             VALUES (?1, ?2, ?3, ?4)",
            params![new_id(), order_id, issue_id, position],
        )?;
    }
    Ok(())
}

pub fn reorder_order(conn: &mut Connection, order_id: &str, issue_ids: &[String]) -> AppResult<()> {
    let tx = conn.transaction()?;
    for (index, issue_id) in issue_ids.iter().enumerate() {
        tx.execute(
            "UPDATE reading_order_items SET position = ?3 WHERE order_id = ?1 AND issue_id = ?2",
            params![order_id, issue_id, index as f64],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub fn next_in_order(conn: &Connection, order_id: &str, issue_id: &str) -> AppResult<Option<NextIssue>> {
    let current: Option<f64> = conn
        .query_row(
            "SELECT position FROM reading_order_items WHERE order_id = ?1 AND issue_id = ?2",
            params![order_id, issue_id],
            |row| row.get(0),
        )
        .optional()?;
    let Some(current) = current else {
        return Ok(None);
    };
    let sql = format!(
        "SELECT {DISPLAY_FIELDS} FROM reading_order_items r
         JOIN issues i ON i.id = r.issue_id
         LEFT JOIN series s ON s.id = i.series_id
         LEFT JOIN library_files f ON f.id = i.file_id
         LEFT JOIN progress p ON p.issue_id = i.id
         WHERE r.order_id = ?1 AND r.position > ?2 ORDER BY r.position LIMIT 1"
    );
    let mut stmt = conn.prepare(&sql)?;
    let row = stmt
        .query_row(params![order_id, current], library::map_issue)
        .optional()?;
    Ok(row.map(|card| NextIssue {
        id: card.id,
        title: card.title,
        number: card.number,
        series_title: card.series_title,
        file_id: card.file_id,
    }))
}

pub fn collections(conn: &Connection) -> AppResult<Vec<Collection>> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.name, c.description, c.cover_file_id,
            (SELECT COUNT(*) FROM collection_items ci WHERE ci.collection_id = c.id),
            c.created_at
         FROM collections c ORDER BY c.name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Collection {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            cover_file_id: row.get(3)?,
            item_count: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn collection_detail(conn: &Connection, collection_id: &str) -> AppResult<CollectionDetail> {
    let config = settings(conn)?;
    let collection = collections(conn)?
        .into_iter()
        .find(|collection| collection.id == collection_id)
        .ok_or_else(|| AppError::not_found("collection"))?;
    let sql = format!(
        "SELECT {DISPLAY_FIELDS} FROM collection_items ci
         JOIN issues i ON i.id = ci.issue_id
         LEFT JOIN series s ON s.id = i.series_id
         LEFT JOIN library_files f ON f.id = i.file_id
         LEFT JOIN progress p ON p.issue_id = i.id
         WHERE ci.collection_id = ?1 ORDER BY ci.position"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([collection_id], library::map_issue)?;
    let mut issues = rows.collect::<Result<Vec<_>, _>>()?;
    apply_spoilers(conn, &config, &mut issues)?;
    Ok(CollectionDetail { collection, items: issues })
}

pub fn create_collection(conn: &Connection, name: &str, description: Option<&str>) -> AppResult<Collection> {
    let id = new_id();
    let timestamp = now();
    conn.execute(
        "INSERT INTO collections (id, name, description, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![id, name, description, timestamp],
    )?;
    Ok(Collection {
        id,
        name: name.into(),
        description: description.map(Into::into),
        cover_file_id: None,
        item_count: 0,
        created_at: timestamp,
    })
}

pub fn update_collection(
    conn: &Connection,
    collection_id: &str,
    name: Option<&str>,
    description: Option<&str>,
    cover_file_id: Option<&str>,
) -> AppResult<Collection> {
    conn.execute(
        "UPDATE collections SET name = coalesce(?2, name), description = coalesce(?3, description),
            cover_file_id = coalesce(?4, cover_file_id),
            updated_at = ?5 WHERE id = ?1",
        params![collection_id, name, description, cover_file_id, now()],
    )?;
    collections(conn)?
        .into_iter()
        .find(|collection| collection.id == collection_id)
        .ok_or_else(|| AppError::not_found("collection"))
}

pub fn delete_collection(conn: &Connection, collection_id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM collections WHERE id = ?1", [collection_id])?;
    Ok(())
}

pub fn add_to_collection(conn: &mut Connection, collection_id: &str, issue_ids: &[String]) -> AppResult<()> {
    let tx = conn.transaction()?;
    let max: f64 = tx
        .query_row(
            "SELECT coalesce(max(position), -1) FROM collection_items WHERE collection_id = ?1",
            [collection_id],
            |row| row.get(0),
        )
        .unwrap_or(-1.0);
    let mut position = max;
    for issue_id in issue_ids {
        position += 1.0;
        tx.execute(
            "INSERT OR IGNORE INTO collection_items (collection_id, issue_id, position)
             VALUES (?1, ?2, ?3)",
            params![collection_id, issue_id, position],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub fn add_series_to_collection(conn: &mut Connection, collection_id: &str, series_id: &str) -> AppResult<()> {
    let ids: Vec<String> = {
        let mut stmt = conn.prepare(
            "SELECT id FROM issues WHERE series_id = ?1 ORDER BY sort_number",
        )?;
        let rows = stmt.query_map([series_id], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    add_to_collection(conn, collection_id, &ids)
}

pub fn remove_from_collection(conn: &Connection, collection_id: &str, issue_ids: &[String]) -> AppResult<()> {
    for issue_id in issue_ids {
        conn.execute(
            "DELETE FROM collection_items WHERE collection_id = ?1 AND issue_id = ?2",
            params![collection_id, issue_id],
        )?;
    }
    Ok(())
}

pub fn reorder_collection(conn: &mut Connection, collection_id: &str, issue_ids: &[String]) -> AppResult<()> {
    let tx = conn.transaction()?;
    for (index, issue_id) in issue_ids.iter().enumerate() {
        tx.execute(
            "UPDATE collection_items SET position = ?3 WHERE collection_id = ?1 AND issue_id = ?2",
            params![collection_id, issue_id, index as f64],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub fn bookmarks(conn: &Connection, issue_id: &str) -> AppResult<Vec<serde_json::Value>> {
    let mut stmt = conn.prepare(
        "SELECT id, page, label, note, created_at FROM bookmarks WHERE issue_id = ?1 ORDER BY page",
    )?;
    let rows = stmt.query_map([issue_id], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, String>(0)?,
            "page": row.get::<_, i64>(1)?,
            "label": row.get::<_, Option<String>>(2)?,
            "note": row.get::<_, Option<String>>(3)?,
            "createdAt": row.get::<_, String>(4)?,
        }))
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn add_bookmark(
    conn: &Connection,
    issue_id: &str,
    page: i64,
    label: Option<&str>,
    note: Option<&str>,
) -> AppResult<serde_json::Value> {
    let id = new_id();
    let timestamp = now();
    conn.execute(
        "INSERT INTO bookmarks (id, issue_id, page, label, note, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, issue_id, page, label, note, timestamp],
    )?;
    Ok(serde_json::json!({
        "id": id,
        "page": page,
        "label": label,
        "note": note,
        "createdAt": timestamp,
    }))
}

pub fn update_bookmark_note(
    conn: &Connection,
    id: &str,
    note: Option<&str>,
) -> AppResult<serde_json::Value> {
    let updated = conn.execute(
        "UPDATE bookmarks SET note = ?2 WHERE id = ?1",
        params![id, note],
    )?;
    if updated == 0 {
        return Err(AppError::not_found("bookmark"));
    }
    let row = conn.query_row(
        "SELECT id, issue_id, page, label, note, created_at FROM bookmarks WHERE id = ?1",
        [id],
        |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "issueId": row.get::<_, String>(1)?,
                "page": row.get::<_, i64>(2)?,
                "label": row.get::<_, Option<String>>(3)?,
                "note": row.get::<_, Option<String>>(4)?,
                "createdAt": row.get::<_, String>(5)?,
            }))
        },
    )?;
    Ok(row)
}

pub fn delete_bookmark(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM bookmarks WHERE id = ?1", [id])?;
    Ok(())
}

pub fn reveal_spoiler(conn: &Connection, issue_id: &str) -> AppResult<()> {
    conn.execute(
        "INSERT OR IGNORE INTO spoiler_reveals (issue_id, revealed_at) VALUES (?1, ?2)",
        params![issue_id, now()],
    )?;
    Ok(())
}

pub fn record_session(conn: &Connection, issue_id: &str, pages: i64, seconds: i64) -> AppResult<()> {
    let ended = chrono::Utc::now();
    let started = ended - chrono::Duration::seconds(seconds.max(1));
    conn.execute(
        "INSERT INTO reading_sessions (id, issue_id, started_at, ended_at, pages)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            new_id(),
            issue_id,
            started.to_rfc3339(),
            ended.to_rfc3339(),
            pages
        ],
    )?;
    Ok(())
}

pub fn reading_history(conn: &Connection, limit: i64) -> AppResult<HistoryPayload> {
    let config = settings(conn)?;
    let limit = limit.clamp(1, 500);
    let mut stmt = conn.prepare(
        "SELECT rs.id, rs.issue_id, i.title, s.title, i.number, i.file_id,
                rs.started_at, rs.ended_at, rs.pages
         FROM reading_sessions rs
         JOIN issues i ON i.id = rs.issue_id
         LEFT JOIN series s ON s.id = i.series_id
         ORDER BY rs.ended_at DESC LIMIT ?1",
    )?;
    let sessions = stmt
        .query_map([limit], |row| {
            Ok(HistorySession {
                id: row.get(0)?,
                issue_id: row.get(1)?,
                title: row.get(2)?,
                series_title: row.get(3)?,
                number: row.get(4)?,
                file_id: row.get(5)?,
                started_at: row.get(6)?,
                ended_at: row.get(7)?,
                pages: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let sql = format!(
        "{} WHERE p.last_read_at IS NOT NULL AND p.status <> 'unread'
         ORDER BY p.last_read_at DESC LIMIT ?1",
        issue_select()
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([limit], library::map_issue)?;
    let mut recent = rows.collect::<Result<Vec<_>, _>>()?;
    apply_spoilers(conn, &config, &mut recent)?;

    Ok(HistoryPayload { sessions, recent })
}

pub fn stats(conn: &Connection) -> AppResult<Stats> {
    let (total_issues, completed, in_progress, total_pages_read, reading_seconds): (i64, i64, i64, i64, i64) =
        conn.query_row(
            "SELECT
                (SELECT COUNT(*) FROM issues),
                (SELECT COUNT(*) FROM progress WHERE status = 'completed'),
                (SELECT COUNT(*) FROM progress WHERE status = 'in-progress'),
                (SELECT coalesce(sum(page), 0) FROM progress),
                (SELECT coalesce(sum(strftime('%s', ended_at) - strftime('%s', started_at)), 0)
                 FROM reading_sessions)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )?;

    let month_start = chrono::Utc::now()
        .format("%Y-%m")
        .to_string();
    let completed_this_month: i64 = conn.query_row(
        "SELECT COUNT(*) FROM progress WHERE status = 'completed'
         AND completed_at LIKE ?1 || '%'",
        [&month_start],
        |row| row.get(0),
    )?;

    let mut stmt = conn.prepare(
        "SELECT coalesce(s.title, i.title),
                COUNT(CASE WHEN p.status = 'completed' THEN 1 END),
                COUNT(i.id)
         FROM issues i
         LEFT JOIN series s ON s.id = i.series_id
         LEFT JOIN progress p ON p.issue_id = i.id
         GROUP BY 1 ORDER BY 2 DESC, 3 DESC LIMIT 8",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(StatSeries {
            name: row.get(0)?,
            read: row.get(1)?,
            total: row.get(2)?,
        })
    })?;
    let top_series = rows.collect::<Result<Vec<_>, _>>()?;

    let mut days_stmt = conn.prepare(
        "SELECT DISTINCT substr(coalesce(last_read_at, updated_at), 1, 10) FROM progress
         WHERE status <> 'unread' ORDER BY 1 DESC LIMIT 400",
    )?;
    let days = days_stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let today = chrono::Utc::now().date_naive();
    let mut streak_days = 0i64;
    let mut cursor = today;
    for day in &days {
        let Ok(date) = chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d") else {
            continue;
        };
        if date == cursor {
            streak_days += 1;
            cursor -= chrono::Duration::days(1);
        } else if date == cursor - chrono::Duration::days(1) && streak_days == 0 {
            streak_days += 1;
            cursor = date - chrono::Duration::days(1);
        } else if date < cursor {
            break;
        }
    }

    let mut status_stmt = conn.prepare(
        "SELECT coalesce(status, 'unread'), COUNT(*) FROM progress GROUP BY 1",
    )?;
    let by_status = status_stmt
        .query_map([], |row| {
            Ok(StatBucket {
                label: row.get(0)?,
                count: row.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Stats {
        total_issues,
        completed,
        in_progress,
        total_pages_read,
        reading_seconds,
        completed_this_month,
        streak_days,
        top_series,
        by_status,
    })
}

pub fn cover_bytes(
    conn: &Connection,
    file_id: &str,
    page: i64,
    max_width: u32,
) -> AppResult<Vec<u8>> {
    let (path, format): (String, String) = conn.query_row(
        "SELECT path, format FROM library_files WHERE id = ?1",
        [file_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let entry: Option<String> = conn
        .query_row(
            "SELECT entry_name FROM file_pages WHERE file_id = ?1 AND page_index = ?2",
            params![file_id, page],
            |row| row.get(0),
        )
        .optional()?;
    let Some(entry) = entry else {
        return Err(AppError::not_found("page"));
    };
    let data = archive::page_bytes(std::path::Path::new(&path), &format, &entry)?;
    archive::thumbnail(&data, max_width)
}

pub fn page_bytes(conn: &Connection, file_id: &str, page: i64) -> AppResult<(Vec<u8>, String)> {
    let (path, format): (String, String) = conn.query_row(
        "SELECT path, format FROM library_files WHERE id = ?1",
        [file_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let entry: String = conn.query_row(
        "SELECT entry_name FROM file_pages WHERE file_id = ?1 AND page_index = ?2",
        params![file_id, page],
        |row| row.get(0),
    )?;
    let data = archive::page_bytes(std::path::Path::new(&path), &format, &entry)?;
    let lower = entry.to_ascii_lowercase();
    let mime = if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".bmp") {
        "image/bmp"
    } else if lower.ends_with(".jp2") {
        "image/jp2"
    } else {
        "image/jpeg"
    };
    Ok((data, mime.into()))
}

/// Applies a reading-direction override to every issue file in a series.
/// `None` clears the per-file override so the series default applies again.
pub fn set_series_direction(
    conn: &mut Connection,
    series_id: &str,
    direction: Option<&str>,
) -> AppResult<i64> {
    if let Some(value) = direction
        && value != "ltr"
        && value != "rtl"
    {
        return Err(AppError::invalid("reading direction must be ltr or rtl"));
    }
    let file_ids: Vec<String> = {
        let mut stmt = conn.prepare(
            "SELECT file_id FROM issues WHERE series_id = ?1 AND file_id IS NOT NULL",
        )?;
        let rows = stmt.query_map([series_id], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    let tx = conn.transaction()?;
    let timestamp = now();
    for file_id in &file_ids {
        tx.execute(
            "INSERT INTO file_preferences (file_id, spread_offset, reading_direction, updated_at)
             VALUES (?1, 0, ?2, ?3)
             ON CONFLICT(file_id) DO UPDATE SET reading_direction = excluded.reading_direction,
                updated_at = excluded.updated_at",
            params![file_id, direction, timestamp],
        )?;
    }
    tx.commit()?;
    Ok(file_ids.len() as i64)
}

/// Marks every issue in a series with the given status. Returns the number of
/// issues touched.
pub fn set_series_status(conn: &mut Connection, series_id: &str, status: &str) -> AppResult<i64> {
    if !["unread", "in-progress", "completed"].contains(&status) {
        return Err(AppError::invalid("unknown status"));
    }
    let ids: Vec<String> = {
        let mut stmt = conn.prepare("SELECT id FROM issues WHERE series_id = ?1")?;
        let rows = stmt.query_map([series_id], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    for id in &ids {
        set_status(conn, id, status)?;
    }
    Ok(ids.len() as i64)
}
