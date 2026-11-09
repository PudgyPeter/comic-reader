use std::io::Write;
use std::path::{Path, PathBuf};

use comic_reader_lib::db::Database;
use comic_reader_lib::library;
use comic_reader_lib::models::{IssuePatch, LibraryFilter, Settings};
use comic_reader_lib::repo;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "panel-test-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_cbz(path: &Path, pages: usize, wide_first: bool) {
    let file = std::fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options: zip::write::FileOptions<()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for index in 0..pages {
        let (width, height) = if index == 0 && wide_first {
            (1600, 1000)
        } else {
            (800, 1200)
        };
        let mut image = image::RgbImage::new(width, height);
        for pixel in image.pixels_mut() {
            *pixel = image::Rgb([(index * 17) as u8, 40, 90]);
        }
        let mut buffer = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image)
            .write_to(&mut buffer, image::ImageFormat::Png)
            .unwrap();
        zip.start_file(format!("page-{:03}.png", index + 1), options)
            .unwrap();
        zip.write_all(&buffer.into_inner()).unwrap();
    }
    zip.finish().unwrap();
}

fn write_cbz_with_info(path: &Path, pages: usize, series: &str, number: &str, title: &str) {
    let file = std::fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options: zip::write::FileOptions<()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for index in 0..pages {
        let mut image = image::RgbImage::new(800, 1200);
        for pixel in image.pixels_mut() {
            *pixel = image::Rgb([(index * 17) as u8, 40, 90]);
        }
        let mut buffer = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image)
            .write_to(&mut buffer, image::ImageFormat::Png)
            .unwrap();
        zip.start_file(format!("page-{:03}.png", index + 1), options)
            .unwrap();
        zip.write_all(&buffer.into_inner()).unwrap();
    }
    let xml = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<ComicInfo>
  <Series>{series}</Series>
  <Number>{number}</Number>
  <Title>{title}</Title>
  <Publisher>IDW Publishing</Publisher>
  <Year>2018</Year>
  <Writer>Ian Flynn</Writer>
  <Penciller>Tyson Hesse</Penciller>
  <Summary>A test issue for the reader.</Summary>
</ComicInfo>"#
    );
    zip.start_file("ComicInfo.xml", options).unwrap();
    zip.write_all(xml.as_bytes()).unwrap();
    zip.finish().unwrap();
}

fn setup(name: &str) -> (PathBuf, Database) {
    let dir = temp_dir(name);
    let db_dir = dir.join("db");
    let db = Database::open(&db_dir).unwrap();
    (dir, db)
}

#[test]
fn imports_cbz_and_reads_pages() {
    let (dir, db) = setup("import");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    let archive = library_dir.join("Sonic the Hedgehog 001.cbz");
    write_cbz_with_info(&archive, 4, "Sonic the Hedgehog", "1", "Fallout");

    let result = db
        .with(|conn| library::import_root(conn, &library_dir))
        .unwrap();
    assert_eq!(result.files_imported, 1);
    assert_eq!(result.issues_created, 1);

    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "series".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    assert_eq!(snapshot.issues.len(), 1);
    let issue = &snapshot.issues[0];
    assert_eq!(issue.title, "Fallout");
    assert_eq!(issue.writer.as_deref(), Some("Ian Flynn"));
    assert_eq!(issue.page_count, 4);
    assert!(!issue.missing);

    let payload = db
        .with(|conn| repo::reader_payload(conn, &issue.id))
        .unwrap();
    assert_eq!(payload.pages.len(), 4);
    assert_eq!(payload.progress.page, 0);

    let (bytes, mime) = db
        .with(|conn| repo::page_bytes(conn, &payload.file_id, 0))
        .unwrap();
    assert!(bytes.len() > 100);
    assert_eq!(mime, "image/png");

    let cover = db
        .with(|conn| repo::cover_bytes(conn, &payload.file_id, 0, 200))
        .unwrap();
    assert_eq!(&cover[..2], &[0xFF, 0xD8]);
}

#[test]
fn progress_resumes_and_completes_at_threshold() {
    let (dir, db) = setup("progress");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    write_cbz(&library_dir.join("Issue 002.cbz"), 10, false);
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();
    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "series".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    let issue = &snapshot.issues[0];

    db.with(|conn| repo::save_progress(conn, &issue.id, 3, None))
        .unwrap();
    let state = db
        .with(|conn| repo::save_progress(conn, &issue.id, 4, None))
        .unwrap();
    assert_eq!(state.status, "in-progress");
    assert_eq!(state.page, 4);

    let done = db
        .with(|conn| repo::save_progress(conn, &issue.id, 9, None))
        .unwrap();
    assert_eq!(done.status, "completed");
    assert!(done.completed_at.is_some());

    let reader = db
        .with(|conn| repo::reader_payload(conn, &issue.id))
        .unwrap();
    assert_eq!(reader.progress.page, 9);
    assert_eq!(reader.progress.status, "completed");
}

#[test]
fn manual_status_and_metadata_edits() {
    let (dir, db) = setup("metadata");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    write_cbz(&library_dir.join("Sonic 003.cbz"), 3, false);
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();
    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "series".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    let issue = &snapshot.issues[0];

    let updated = db
        .with(|conn| {
            repo::update_issue(
                conn,
                &issue.id,
                &IssuePatch {
                    title: Some("Renamed".into()),
                    number: Some("3".into()),
                    volume: None,
                    publisher: Some("Archie".into()),
                    release_date: Some("2019-04-01".into()),
                    writer: Some("Writer".into()),
                    artist: None,
                    cover_artist: None,
                    description: Some("Summary".into()),
                    series_id: None,
                    series_title: None,
                    ..Default::default()
                },
            )
        })
        .unwrap();
    assert_eq!(updated.title, "Renamed");
    assert_eq!(updated.publisher.as_deref(), Some("Archie"));

    let completed = db
        .with(|conn| repo::set_status(conn, &issue.id, "completed"))
        .unwrap();
    assert_eq!(completed.status, "completed");
    assert_eq!(completed.last_page, 2);

    let unread = db
        .with(|conn| repo::set_status(conn, &issue.id, "unread"))
        .unwrap();
    assert_eq!(unread.status, "unread");
    assert_eq!(unread.last_page, 0);
}

#[test]
fn collections_orders_and_search() {
    let (dir, db) = setup("organise");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    for index in 1..=3 {
        let path = library_dir.join(format!("Sonic the Hedgehog {index:03}.cbz"));
        write_cbz_with_info(&path, 3, "Sonic the Hedgehog", &index.to_string(), &format!("Issue {index}"));
    }
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();

    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "number".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    assert_eq!(snapshot.issues.len(), 3);
    assert_eq!(snapshot.series.len(), 1);

    let order = db
        .with(|conn| repo::build_publication_order(conn, "Publication"))
        .unwrap();
    assert_eq!(order.items.len(), 3);
    let ids: Vec<String> = order.items.iter().map(|item| item.issue_id.clone()).collect();
    db.with(|conn| repo::reorder_order(conn, &order.order.id, &ids))
        .unwrap();

    let next = db
        .with(|conn| repo::next_in_order(conn, &order.order.id, &ids[0]))
        .unwrap();
    assert!(next.is_some());

    let collection = db
        .with(|conn| repo::create_collection(conn, "Best of", Some("Favourites")))
        .unwrap();
    db.with(|conn| repo::add_to_collection(conn, &collection.id, &ids[..2]))
        .unwrap();
    let detail = db
        .with(|conn| repo::collection_detail(conn, &collection.id))
        .unwrap();
    assert_eq!(detail.items.len(), 2);

    let search = db
        .with(|conn| repo::search(conn, "Issue 2", 20))
        .unwrap();
    assert!(!search.issues.is_empty());
}

#[test]
fn spoilers_hide_unread_descriptions() {
    let (dir, db) = setup("spoilers");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    for index in 1..=3 {
        let path = library_dir.join(format!("Sonic {index:03}.cbz"));
        write_cbz_with_info(&path, 2, "Sonic", &index.to_string(), &format!("Issue {index}"));
    }
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();
    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "number".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    let first = &snapshot.issues[0];
    db.with(|conn| repo::save_progress(conn, &first.id, 1, Some(0.5)))
        .unwrap();

    let mut settings = db.with(|conn| repo::settings(conn)).unwrap();
    settings.spoiler_protection = true;
    db.with(|conn| repo::save_settings(conn, &settings)).unwrap();

    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "number".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    let protected = snapshot
        .issues
        .iter()
        .find(|issue| issue.title == "Issue 3")
        .unwrap();
    assert!(protected.spoiler_protected);
    assert!(protected.description.is_none());

    db.with(|conn| repo::reveal_spoiler(conn, &protected.id))
        .unwrap();
    let fresh = db
        .with(|conn| repo::issue(conn, &protected.id))
        .unwrap();
    assert!(!fresh.spoiler_protected);
    assert_eq!(fresh.description.as_deref(), Some("A test issue for the reader."));
}

#[test]
fn settings_roundtrip_and_parse_helpers() {
    let (_dir, db) = setup("settings");
    let defaults = Settings::default();
    let mut shortcuts = defaults.shortcuts.clone();
    shortcuts.insert("nextPage".into(), "K".into());
    let settings = Settings {
        background: "#123456".into(),
        completion_threshold: 0.5,
        shortcuts,
        ..defaults
    };
    let saved = db.with(|conn| repo::save_settings(conn, &settings)).unwrap();
    assert_eq!(saved.background, "#123456");
    assert_eq!(saved.completion_threshold, 0.5);
    assert_eq!(saved.shortcuts.get("nextPage").map(String::as_str), Some("K"));
}

#[test]
fn filename_parsing_handles_common_patterns() {
    use comic_reader_lib::metadata::parse_filename;
    let parsed = parse_filename("Sonic the Hedgehog #032 (2018).cbz");
    assert_eq!(parsed.series.trim(), "Sonic the Hedgehog");
    assert_eq!(parsed.number, "32");
    assert_eq!(parsed.year.as_deref(), Some("2018"));

    let parsed = parse_filename("Sonic_Universe_Issue_05.cbz");
    assert_eq!(parsed.series.trim(), "Sonic Universe");
    assert_eq!(parsed.number, "5");
}

#[test]
fn rescan_is_idempotent() {
    let (dir, db) = setup("rescan");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    write_cbz(&library_dir.join("A 001.cbz"), 2, false);
    let first = db.with(|conn| library::import_root(conn, &library_dir)).unwrap();
    assert_eq!(first.files_imported, 1);
    let second = db.with(library::rescan).unwrap();
    let imported: i64 = second.iter().map(|entry| entry.files_imported).sum();
    assert_eq!(imported, 0);
    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "series".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    assert_eq!(snapshot.issues.len(), 1);
}

#[test]
fn duplicate_imports_warn_but_still_import() {
    let (dir, db) = setup("dupes");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    write_cbz_with_info(&library_dir.join("Sonic 001.cbz"), 2, "Sonic", "1", "Issue 1");
    let first = db.with(|conn| library::import_root(conn, &library_dir)).unwrap();
    assert!(first.warnings.iter().all(|warning| warning.kind != "duplicate"));

    write_cbz_with_info(&library_dir.join("Sonic 001 variant.cbz"), 2, "Sonic", "1", "Issue 1 variant");
    let second = db.with(|conn| library::import_root(conn, &library_dir)).unwrap();
    assert_eq!(second.files_imported, 1);
    assert!(second
        .warnings
        .iter()
        .any(|warning| warning.kind == "duplicate"));
}

#[test]
fn reading_order_drives_spoiler_frontier() {
    let (dir, db) = setup("order-spoilers");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    for index in 1..=4 {
        let path = library_dir.join(format!("Sonic {index:03}.cbz"));
        write_cbz_with_info(&path, 2, "Sonic", &index.to_string(), &format!("Issue {index}"));
    }
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();

    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "number".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    let issues: Vec<_> = snapshot.issues.iter().collect();

    // Build an order that deliberately puts issue 4 second, then mark it and the
    // first item complete. Issue 3 sits later and must stay protected even though
    // its series progress is lower.
    let order = db
        .with(|conn| repo::build_publication_order(conn, "Reversed"))
        .unwrap();
    let mut ids: Vec<String> = order.items.iter().map(|item| item.issue_id.clone()).collect();
    let fourth = issues.iter().find(|issue| issue.title == "Issue 4").unwrap();
    let position = ids.iter().position(|id| id == &fourth.id).unwrap();
    ids.swap(1, position);
    db.with(|conn| repo::reorder_order(conn, &order.order.id, &ids))
        .unwrap();
    db.with(|conn| repo::save_progress(conn, &ids[0], 1, Some(0.5)))
        .unwrap();
    db.with(|conn| repo::save_progress(conn, &ids[1], 1, Some(0.5)))
        .unwrap();

    let mut settings = db.with(|conn| repo::settings(conn)).unwrap();
    settings.spoiler_protection = true;
    settings.spoiler_order_id = Some(order.order.id.clone());
    db.with(|conn| repo::save_settings(conn, &settings)).unwrap();

    let detail = db
        .with(|conn| repo::order_detail(conn, &order.order.id))
        .unwrap();
    let third = detail
        .items
        .iter()
        .find(|item| item.issue.as_ref().is_some_and(|issue| issue.title == "Issue 3"))
        .unwrap()
        .issue
        .as_ref()
        .unwrap();
    assert!(third.spoiler_protected, "issue after the order frontier must be protected");

    let fourth_detail = db.with(|conn| repo::issue(conn, &fourth.id)).unwrap();
    assert!(!fourth_detail.spoiler_protected);
}

#[test]
fn spread_offset_shifts_pairing() {
    let (dir, db) = setup("offset");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    write_cbz_with_info(&library_dir.join("Sonic 001.cbz"), 6, "Sonic", "1", "Issue 1");
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();
    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "number".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    let issue = &snapshot.issues[0];
    assert_eq!(issue.id.len(), 36);

    let payload = db.with(|conn| repo::reader_payload(conn, &issue.id)).unwrap();
    assert_eq!(payload.spread_offset, 0);

    let offset = db
        .with(|conn| repo::set_spread_offset(conn, &issue.id, 1))
        .unwrap();
    assert_eq!(offset, 1);
    let payload = db.with(|conn| repo::reader_payload(conn, &issue.id)).unwrap();
    assert_eq!(payload.spread_offset, 1);

    // Wraps modulo 2 rather than accumulating.
    let offset = db
        .with(|conn| repo::set_spread_offset(conn, &issue.id, 3))
        .unwrap();
    assert_eq!(offset, 1);
}

#[test]
fn reading_direction_resolves_file_then_series_then_global() {
    let (dir, db) = setup("direction");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    write_cbz_with_info(&library_dir.join("Sonic 001.cbz"), 3, "Sonic", "1", "Issue 1");
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();

    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "number".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    let issue = &snapshot.issues[0];
    let series_id = issue.series_id.clone().unwrap();

    // Global default wins when neither override exists.
    let payload = db.with(|conn| repo::reader_payload(conn, &issue.id)).unwrap();
    assert_eq!(payload.reading_direction, "ltr");

    // Series override applies to every issue in the series.
    db.with(|conn| {
        repo::update_series(
            conn,
            &series_id,
            &comic_reader_lib::models::SeriesPatch {
                reading_direction: Some("rtl".into()),
                ..Default::default()
            },
        )
    })
    .unwrap();
    let payload = db.with(|conn| repo::reader_payload(conn, &issue.id)).unwrap();
    assert_eq!(payload.reading_direction, "rtl");

    // File override is more specific than the series.
    db.with(|conn| repo::set_direction_override(conn, &issue.id, Some("ltr")))
        .unwrap();
    let payload = db.with(|conn| repo::reader_payload(conn, &issue.id)).unwrap();
    assert_eq!(payload.reading_direction, "ltr");

    // Clearing the file override falls back to the series again.
    db.with(|conn| repo::set_direction_override(conn, &issue.id, None))
        .unwrap();
    let payload = db.with(|conn| repo::reader_payload(conn, &issue.id)).unwrap();
    assert_eq!(payload.reading_direction, "rtl");

    // Invalid directions are rejected rather than stored.
    assert!(db
        .with(|conn| repo::set_direction_override(conn, &issue.id, Some("sideways")))
        .is_err());
}

#[test]
fn reading_history_lists_sessions_and_recent_issues() {
    let (dir, db) = setup("history");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    write_cbz_with_info(&library_dir.join("Sonic 001.cbz"), 4, "Sonic", "1", "Issue 1");
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();
    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "number".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    let issue = &snapshot.issues[0];

    db.with(|conn| repo::record_session(conn, &issue.id, 5, 90))
        .unwrap();
    db.with(|conn| repo::save_progress(conn, &issue.id, 2, Some(0.99)))
        .unwrap();

    let history = db.with(|conn| repo::reading_history(conn, 10)).unwrap();
    assert_eq!(history.sessions.len(), 1);
    assert_eq!(history.sessions[0].title, "Issue 1");
    assert_eq!(history.sessions[0].pages, 5);
    assert_eq!(history.recent.len(), 1);
    assert_eq!(history.recent[0].id, issue.id);
}

#[test]
fn bookmark_notes_can_be_updated_and_searched() {
    let (dir, db) = setup("bookmark-note");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    write_cbz_with_info(&library_dir.join("Sonic 001.cbz"), 3, "Sonic", "1", "Issue 1");
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();
    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "number".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    let issue = &snapshot.issues[0];

    let mark = db
        .with(|conn| repo::add_bookmark(conn, &issue.id, 2, Some("Fight scene"), Some("draft note")))
        .unwrap();
    let mark_id = mark["id"].as_str().unwrap().to_string();

    let updated = db
        .with(|conn| repo::update_bookmark_note(conn, &mark_id, Some("remember the lettering")))
        .unwrap();
    assert_eq!(updated["note"].as_str(), Some("remember the lettering"));

    let hits = db.with(|conn| repo::search(conn, "lettering", 10)).unwrap();
    assert_eq!(hits.issues.len(), 1);
    assert_eq!(hits.issues[0].id, issue.id);

    assert!(db
        .with(|conn| repo::update_bookmark_note(conn, "missing-bookmark", Some("nope")))
        .is_err());
}

#[test]
fn import_reports_progress_and_honours_cancel() {
    let (dir, db) = setup("import-progress");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    write_cbz_with_info(&library_dir.join("Sonic 001.cbz"), 2, "Sonic", "1", "Issue 1");
    write_cbz_with_info(&library_dir.join("Tails 001.cbz"), 2, "Tails", "1", "Issue 1");

    let mut stages = Vec::new();
    let result = db
        .with(|conn| {
            library::import_root_with_progress(
                conn,
                &library_dir,
                &library::ImportControl::new(),
                |progress| stages.push(progress.stage.clone()),
            )
        })
        .unwrap();
    assert_eq!(result.files_imported, 2);
    assert!(stages.contains(&"importing".to_string()));
    assert_eq!(stages.last().map(String::as_str), Some("done"));

    // A pre-cancelled control aborts before touching the database.
    let control = library::ImportControl::new();
    control.cancel();
    let cancelled = db.with(|conn| {
        library::import_root_with_progress(conn, &library_dir, &control, |_| {})
    });
    assert!(cancelled.is_err());

    // Reset clears the flag so the same control can be reused.
    control.reset();
    assert!(!control.is_cancelled());
}

#[test]
fn missing_issue_detection_finds_gaps_and_respects_overrides() {
    let (dir, db) = setup("missing");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    for number in ["1", "2", "5", "7"] {
        write_cbz_with_info(
            &library_dir.join(format!("Sonic {number:0>3}.cbz")),
            2,
            "Sonic",
            number,
            "Issue",
        );
    }
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();

    let series_id = db
        .with(|conn| repo::series_summaries(conn))
        .unwrap()
        .into_iter()
        .find(|series| series.title == "Sonic")
        .unwrap()
        .id;

    let detail = db.with(|conn| repo::series_detail(conn, &series_id)).unwrap();
    let numbers: Vec<&str> = detail.missing.iter().map(|entry| entry.number.as_str()).collect();
    assert_eq!(numbers, vec!["3", "4", "6"]);

    let summary = db
        .with(|conn| repo::series_summaries(conn))
        .unwrap()
        .into_iter()
        .find(|series| series.id == series_id)
        .unwrap();
    assert_eq!(summary.missing_count, 3);

    // Marking a gap "not required" removes it from the computed list.
    db.with(|conn| repo::set_missing_reason(conn, &series_id, "4", Some("not-required")))
        .unwrap();
    let detail = db.with(|conn| repo::series_detail(conn, &series_id)).unwrap();
    let numbers: Vec<&str> = detail.missing.iter().map(|entry| entry.number.as_str()).collect();
    assert_eq!(numbers, vec!["3", "6"]);

    // Clearing the override brings it back.
    db.with(|conn| repo::set_missing_reason(conn, &series_id, "4", None))
        .unwrap();
    let detail = db.with(|conn| repo::series_detail(conn, &series_id)).unwrap();
    assert!(detail.missing.iter().any(|entry| entry.number == "4"));

    let summary = db
        .with(|conn| repo::series_summaries(conn))
        .unwrap()
        .into_iter()
        .find(|series| series.id == series_id)
        .unwrap();
    assert_eq!(summary.missing_count, 3);

    assert!(db
        .with(|conn| repo::set_missing_reason(conn, &series_id, "9", Some("bogus")))
        .is_err());
}

#[test]
fn variants_link_to_a_base_issue_and_cannot_cycle() {
    let (dir, db) = setup("variants");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    write_cbz_with_info(&library_dir.join("Sonic 001.cbz"), 2, "Sonic", "1", "Issue 1");
    write_cbz_with_info(&library_dir.join("Sonic 001 variant.cbz"), 3, "Sonic", "1", "Issue 1 Variant");
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();

    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "number".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    assert_eq!(snapshot.issues.len(), 2);
    let base = snapshot
        .issues
        .iter()
        .find(|issue| issue.title == "Issue 1")
        .unwrap()
        .clone();
    let variant = snapshot
        .issues
        .iter()
        .find(|issue| issue.title == "Issue 1 Variant")
        .unwrap()
        .clone();

    db.with(|conn| {
        repo::update_issue(
            conn,
            &variant.id,
            &IssuePatch {
                variant_of: Some(base.id.clone()),
                edition: Some("Variant cover B".into()),
                ..Default::default()
            },
        )
    })
    .unwrap();

    let base_after = db.with(|conn| repo::issue(conn, &base.id)).unwrap();
    assert_eq!(base_after.variant_count, 1);
    let variant_after = db.with(|conn| repo::issue(conn, &variant.id)).unwrap();
    assert_eq!(variant_after.variant_of.as_deref(), Some(base.id.as_str()));
    assert_eq!(variant_after.edition.as_deref(), Some("Variant cover B"));

    // Pointing the base at its own variant would form a cycle.
    assert!(db
        .with(|conn| {
            repo::update_issue(
                conn,
                &base.id,
                &IssuePatch {
                    variant_of: Some(variant.id.clone()),
                    ..Default::default()
                },
            )
        })
        .is_err());

    // An empty string clears the link.
    db.with(|conn| {
        repo::update_issue(
            conn,
            &variant.id,
            &IssuePatch {
                variant_of: Some(String::new()),
                ..Default::default()
            },
        )
    })
    .unwrap();
    let variant_after = db.with(|conn| repo::issue(conn, &variant.id)).unwrap();
    assert!(variant_after.variant_of.is_none());
}

#[test]
fn series_bulk_direction_and_status_apply_to_every_issue() {
    let (dir, db) = setup("series-bulk");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    for index in 1..=3 {
        write_cbz_with_info(
            &library_dir.join(format!("Sonic {index:03}.cbz")),
            3,
            "Sonic",
            &index.to_string(),
            &format!("Issue {index}"),
        );
    }
    write_cbz_with_info(
        &library_dir.join("Shadow 001.cbz"),
        2,
        "Shadow",
        "1",
        "Shadow 1",
    );
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();

    let series = db.with(|conn| repo::series_summaries(conn)).unwrap();
    let sonic = series.iter().find(|entry| entry.title == "Sonic").unwrap();

    let touched = db
        .with(|conn| repo::set_series_status(conn, &sonic.id, "completed"))
        .unwrap();
    assert_eq!(touched, 3);

    let detail = db.with(|conn| repo::series_detail(conn, &sonic.id)).unwrap();
    assert!(detail.issues.iter().all(|issue| issue.status == "completed"));

    // The other series is untouched.
    let other = series.iter().find(|entry| entry.title == "Shadow").unwrap();
    let other_detail = db.with(|conn| repo::series_detail(conn, &other.id)).unwrap();
    assert!(other_detail.issues.iter().all(|issue| issue.status != "completed"));

    let files = db
        .with(|conn| repo::set_series_direction(conn, &sonic.id, Some("rtl")))
        .unwrap();
    assert_eq!(files, 3);
    let payload = db
        .with(|conn| repo::reader_payload(conn, &detail.issues[0].id))
        .unwrap();
    assert_eq!(payload.reading_direction, "rtl");

    // Clearing the per-file override falls back to the series/global default.
    let cleared = db
        .with(|conn| repo::set_series_direction(conn, &sonic.id, None))
        .unwrap();
    assert_eq!(cleared, 3);
    let payload = db
        .with(|conn| repo::reader_payload(conn, &detail.issues[0].id))
        .unwrap();
    assert_ne!(payload.reading_direction, "rtl");

    assert!(db
        .with(|conn| repo::set_series_status(conn, &sonic.id, "bogus"))
        .is_err());
}

#[test]
fn generated_orders_skip_alternate_covers() {
    let (dir, db) = setup("variant-orders");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    write_cbz_with_info(&library_dir.join("Sonic 001.cbz"), 2, "Sonic", "1", "Issue 1");
    write_cbz_with_info(&library_dir.join("Sonic 002.cbz"), 2, "Sonic", "2", "Issue 2");
    write_cbz_with_info(
        &library_dir.join("Sonic 001 variant.cbz"),
        3,
        "Sonic",
        "1",
        "Issue 1 Variant",
    );
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();

    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "number".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    let base = snapshot.issues.iter().find(|issue| issue.title == "Issue 1").unwrap();
    let variant = snapshot
        .issues
        .iter()
        .find(|issue| issue.title == "Issue 1 Variant")
        .unwrap();

    db.with(|conn| {
        repo::update_issue(
            conn,
            &variant.id,
            &IssuePatch {
                variant_of: Some(base.id.clone()),
                edition: Some("Cover B".into()),
                ..Default::default()
            },
        )
    })
    .unwrap();

    let order = db
        .with(|conn| repo::build_publication_order(conn, "Publication"))
        .unwrap();
    assert_eq!(order.items.len(), 2, "alternate cover must not add a read");
    assert!(order.items.iter().all(|item| item.issue_id != variant.id));
}

#[test]
fn series_cover_override_wins_over_first_issue() {
    let (dir, db) = setup("series-cover");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    write_cbz_with_info(&library_dir.join("Sonic 001.cbz"), 2, "Sonic", "1", "Issue 1");
    write_cbz_with_info(&library_dir.join("Sonic 002.cbz"), 2, "Sonic", "2", "Issue 2");
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();

    let series = db.with(|conn| repo::series_summaries(conn)).unwrap();
    let sonic = series.iter().find(|entry| entry.title == "Sonic").unwrap();
    let default_cover = sonic.cover_file_id.clone().unwrap();

    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "number".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    let second = snapshot
        .issues
        .iter()
        .find(|issue| issue.number == "2")
        .unwrap()
        .clone();
    db.with(|conn| {
        repo::update_series(
            conn,
            &sonic.id,
            &comic_reader_lib::models::SeriesPatch {
                cover_file_id: Some(second.file_id.clone().unwrap()),
                ..Default::default()
            },
        )
    })
    .unwrap();

    let series = db.with(|conn| repo::series_summaries(conn)).unwrap();
    let sonic = series.iter().find(|entry| entry.title == "Sonic").unwrap();
    assert_ne!(sonic.cover_file_id, Some(default_cover));
    assert_eq!(sonic.cover_file_id, second.file_id);
}

// --- Minimal RAR4 (store-only) writer, just enough to exercise CBR reading ---

fn crc32_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    for (index, slot) in table.iter_mut().enumerate() {
        let mut value = index as u32;
        for _ in 0..8 {
            value = if value & 1 != 0 {
                0xEDB8_8320 ^ (value >> 1)
            } else {
                value >> 1
            };
        }
        *slot = value;
    }
    table
}

fn crc32(data: &[u8]) -> u32 {
    let table = crc32_table();
    let mut value = 0xFFFF_FFFFu32;
    for byte in data {
        value = table[((value ^ *byte as u32) & 0xFF) as usize] ^ (value >> 8);
    }
    value ^ 0xFFFF_FFFF
}

fn png_bytes(width: u32, height: u32, color: [u8; 3]) -> Vec<u8> {
    let mut image = image::RgbImage::new(width, height);
    for pixel in image.pixels_mut() {
        *pixel = image::Rgb(color);
    }
    let mut buffer = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image)
        .write_to(&mut buffer, image::ImageFormat::Png)
        .unwrap();
    buffer.into_inner()
}

fn write_rar4_store(path: &Path, members: &[(&str, Vec<u8>)]) {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"Rar!\x1a\x07\x00");

    let push_header = |out: &mut Vec<u8>, body: &[u8]| {
        let crc = (crc32(body) & 0xFFFF) as u16;
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(body);
    };

    // Main archive header.
    let main_body = {
        let mut body = vec![0x73u8];
        body.extend_from_slice(&0u16.to_le_bytes()); // flags
        body.extend_from_slice(&13u16.to_le_bytes()); // header size
        body.extend_from_slice(&0u16.to_le_bytes()); // reserved
        body.extend_from_slice(&0u32.to_le_bytes()); // reserved
        body
    };
    push_header(&mut out, &main_body);

    for (name, data) in members {
        let name_bytes = name.as_bytes();
        let header_size = 32 + name_bytes.len();
        let mut body = vec![0x74u8]; // file header
        body.extend_from_slice(&0u16.to_le_bytes()); // flags
        body.extend_from_slice(&(header_size as u16).to_le_bytes());
        body.extend_from_slice(&(data.len() as u32).to_le_bytes()); // pack size
        body.extend_from_slice(&(data.len() as u32).to_le_bytes()); // unpacked size
        body.push(2); // host os: unix
        body.extend_from_slice(&crc32(data).to_le_bytes()); // file crc
        body.extend_from_slice(&0x5021_0000u32.to_le_bytes()); // dos time
        body.push(20); // unpack version
        body.push(0x30); // method: store
        body.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        body.extend_from_slice(&0o644u32.to_le_bytes()); // attributes
        body.extend_from_slice(name_bytes);
        push_header(&mut out, &body);
        out.extend_from_slice(data);
    }

    std::fs::write(path, out).unwrap();
}

#[test]
fn imports_cbr_archives() {
    let (dir, db) = setup("cbr");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    let archive = library_dir.join("Sonic Archie 001.cbr");
    let members = [
        ("page-001.png".to_string(), png_bytes(300, 450, [200, 40, 40])),
        ("page-002.png".to_string(), png_bytes(300, 450, [40, 200, 40])),
        ("page-003.png".to_string(), png_bytes(300, 450, [40, 40, 200])),
    ];
    let refs: Vec<(&str, Vec<u8>)> = members
        .iter()
        .map(|(name, data)| (name.as_str(), data.clone()))
        .collect();
    write_rar4_store(&archive, &refs);

    let result = db
        .with(|conn| library::import_root(conn, &library_dir))
        .unwrap();
    assert_eq!(result.files_imported, 1, "warnings: {:?}", result.warnings);

    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "number".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    let issue = &snapshot.issues[0];
    assert_eq!(issue.page_count, 3);
    assert_eq!(issue.title, "Sonic Archie #1");
    assert!(!issue.missing);

    let payload = db
        .with(|conn| repo::reader_payload(conn, &issue.id))
        .unwrap();
    assert_eq!(payload.pages.len(), 3);
    assert_eq!(payload.pages[0].width, 300);

    let (bytes, mime) = db
        .with(|conn| repo::page_bytes(conn, &payload.file_id, 1))
        .unwrap();
    assert!(bytes.len() > 50);
    assert_eq!(mime, "image/png");
}

#[test]
fn imports_image_folder_as_issue() {
    let (dir, db) = setup("folder");
    let library_dir = dir.join("library");
    let comic_dir = library_dir.join("Loose Pages 004");
    std::fs::create_dir_all(&comic_dir).unwrap();
    std::fs::write(comic_dir.join("page10.png"), png_bytes(200, 300, [10, 20, 30])).unwrap();
    std::fs::write(comic_dir.join("page2.png"), png_bytes(200, 300, [30, 20, 10])).unwrap();
    std::fs::write(comic_dir.join("page1.png"), png_bytes(200, 300, [20, 10, 30])).unwrap();
    std::fs::write(comic_dir.join("notes.txt"), b"ignore me").unwrap();

    let result = db
        .with(|conn| library::import_root(conn, &library_dir))
        .unwrap();
    assert_eq!(result.files_imported, 1, "warnings: {:?}", result.warnings);

    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "number".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    let issue = &snapshot.issues[0];
    assert_eq!(issue.page_count, 3);

    let payload = db
        .with(|conn| repo::reader_payload(conn, &issue.id))
        .unwrap();
    // Natural ordering: page1, page2, page10.
    let names: Vec<&str> = payload.pages.iter().map(|page| page.name.as_str()).collect();
    assert_eq!(names, vec!["page1.png", "page2.png", "page10.png"]);
}

fn build_pdf(path: &Path, images: &[Vec<u8>], width: u32, height: u32) {
    // Builds a PDF with one DCTDecode image XObject per page.
    let mut objects: Vec<Vec<u8>> = Vec::new();
    // 1: catalog, 2: pages, then per page: page dict + image xobject + content
    let page_count = images.len();
    let page_ids: Vec<usize> = (0..page_count).map(|index| 4 + index * 3).collect();

    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    let kids = page_ids
        .iter()
        .map(|id| format!("{id} 0 R"))
        .collect::<Vec<_>>()
        .join(" ");
    objects.push(
        format!("<< /Type /Pages /Count {page_count} /Kids [{kids}] >>").into_bytes(),
    );
    // Object 3 is intentionally unused to keep numbering simple.

    for (index, jpeg) in images.iter().enumerate() {
        let page_id = page_ids[index];
        let image_id = page_id + 1;
        let content_id = page_id + 2;
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] \
                 /Resources << /XObject << /Im0 {image_id} 0 R >> >> /Contents {content_id} 0 R >>"
            )
            .into_bytes(),
        );
        let mut image_object = format!(
            "<< /Type /XObject /Subtype /Image /Width {width} /Height {height} \
             /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
            jpeg.len()
        )
        .into_bytes();
        image_object.extend_from_slice(jpeg);
        image_object.extend_from_slice(b"\nendstream");
        objects.push(image_object);
        let content = format!("q {width} 0 0 {height} 0 0 cm /Im0 Do Q");
        objects.push(
            format!(
                "<< /Length {} >>\nstream\n{}\nendstream",
                content.len(),
                content
            )
            .into_bytes(),
        );
    }
    // Pad the reserved object 3 slot so numbering lines up.
    objects.insert(2, b"<< >>".to_vec());

    let mut out: Vec<u8> = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        let id = index + 1;
        offsets.push(out.len());
        out.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_start = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for offset in &offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_start}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    std::fs::write(path, out).unwrap();
}

fn jpeg_bytes(width: u32, height: u32, color: [u8; 3]) -> Vec<u8> {
    let mut image = image::RgbImage::new(width, height);
    for pixel in image.pixels_mut() {
        *pixel = image::Rgb(color);
    }
    let mut buffer = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image)
        .write_to(&mut buffer, image::ImageFormat::Jpeg)
        .unwrap();
    buffer.into_inner()
}

#[test]
fn imports_image_pdf() {
    let (dir, db) = setup("pdf");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    let pdf = library_dir.join("Sonic Mega Drive 001.pdf");
    let images = vec![
        jpeg_bytes(400, 600, [200, 60, 60]),
        jpeg_bytes(400, 600, [60, 200, 60]),
        jpeg_bytes(400, 600, [60, 60, 200]),
    ];
    build_pdf(&pdf, &images, 400, 600);

    let result = db
        .with(|conn| library::import_root(conn, &library_dir))
        .unwrap();
    assert_eq!(result.files_imported, 1, "warnings: {:?}", result.warnings);

    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "number".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    let issue = &snapshot.issues[0];
    assert_eq!(issue.page_count, 3);
    assert_eq!(issue.title, "Sonic Mega Drive #1");

    let payload = db
        .with(|conn| repo::reader_payload(conn, &issue.id))
        .unwrap();
    assert_eq!(payload.pages[0].width, 400);
    assert_eq!(payload.pages[0].height, 600);

    let (bytes, mime) = db
        .with(|conn| repo::page_bytes(conn, &payload.file_id, 2))
        .unwrap();
    assert_eq!(&bytes[..2], &[0xFF, 0xD8]);
    assert_eq!(mime, "image/jpeg");
}


#[test]
fn mark_to_page_sets_position_and_completes_on_last_page() {
    let (dir, db) = setup("mark-to-page");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    write_cbz_with_info(&library_dir.join("Sonic 001.cbz"), 5, "Sonic", "1", "Issue 1");
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();
    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "number".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    let issue = &snapshot.issues[0];

    let state = db.with(|conn| repo::mark_to_page(conn, &issue.id, 2)).unwrap();
    assert_eq!(state.page, 2);
    assert_eq!(state.status, "in-progress");

    // Marking backwards from a completed issue keeps it completed.
    db.with(|conn| repo::mark_to_page(conn, &issue.id, 4)).unwrap();
    let state = db.with(|conn| repo::mark_to_page(conn, &issue.id, 1)).unwrap();
    assert_eq!(state.status, "completed");

    let state = db.with(|conn| repo::mark_to_page(conn, &issue.id, 99)).unwrap();
    assert_eq!(state.page, 4);
    assert_eq!(state.status, "completed");

    let card = db.with(|conn| repo::issue(conn, &issue.id)).unwrap();
    assert_eq!(card.status, "completed");
}

#[test]
fn scroll_offset_survives_saves_and_is_clamped() {
    let (dir, db) = setup("scroll-offset");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    write_cbz_with_info(&library_dir.join("Sonic 001.cbz"), 6, "Sonic", "1", "Issue 1");
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();
    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "number".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    let issue = &snapshot.issues[0];

    db.with(|conn| repo::save_reading_position(conn, &issue.id, 3, Some(0.4), None))
        .unwrap();
    let reader = db.with(|conn| repo::reader_payload(conn, &issue.id)).unwrap();
    assert_eq!(reader.progress.page, 3);
    assert!((reader.progress.scroll_offset - 0.4).abs() < 0.001);

    // A save without an offset (paged modes) keeps the stored continuous offset.
    db.with(|conn| repo::save_reading_position(conn, &issue.id, 4, None, None))
        .unwrap();
    let reader = db.with(|conn| repo::reader_payload(conn, &issue.id)).unwrap();
    assert!((reader.progress.scroll_offset - 0.4).abs() < 0.001);

    // Out-of-range values are clamped rather than trusted.
    db.with(|conn| repo::save_reading_position(conn, &issue.id, 3, Some(9.0), None))
        .unwrap();
    let reader = db.with(|conn| repo::reader_payload(conn, &issue.id)).unwrap();
    assert!((reader.progress.scroll_offset - 1.0).abs() < 0.001);
}

#[test]
fn open_sessions_are_listed_as_interrupted_until_resumed() {
    let (dir, db) = setup("session-resume");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    write_cbz_with_info(&library_dir.join("Sonic 001.cbz"), 4, "Sonic", "1", "Issue 1");
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();
    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "number".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    let issue = &snapshot.issues[0];

    let session = db
        .with(|conn| repo::start_session(conn, &issue.id, 0))
        .unwrap();
    db.with(|conn| repo::touch_session(conn, &session, 3, 2)).unwrap();

    let history = db.with(|conn| repo::reading_history(conn, 10)).unwrap();
    assert_eq!(history.sessions.len(), 0, "open sessions are not closed history");
    assert_eq!(history.interrupted.len(), 1);
    assert_eq!(history.interrupted[0].last_page, 2);
    assert_eq!(history.interrupted[0].pages, 3);

    db.with(|conn| repo::resume_session(conn, &session)).unwrap();
    let history = db.with(|conn| repo::reading_history(conn, 10)).unwrap();
    assert_eq!(history.interrupted.len(), 0);
    assert_eq!(history.sessions.len(), 1);
    assert!(history.sessions[0].resumed_at.is_some());

    // Closing a session records the final position.
    let session = db
        .with(|conn| repo::start_session(conn, &issue.id, 1))
        .unwrap();
    db.with(|conn| repo::close_session(conn, &session, 2, 3)).unwrap();
    let history = db.with(|conn| repo::reading_history(conn, 10)).unwrap();
    assert_eq!(history.sessions.len(), 2);
    assert_eq!(history.sessions[0].last_page, 3);
}

#[test]
fn import_preview_proposes_metadata_and_commit_applies_corrections() {
    let (dir, db) = setup("import-review");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    write_cbz_with_info(
        &library_dir.join("Sonic 001.cbz"),
        3,
        "Wrong Series",
        "1",
        "Wrong Title",
    );

    let control = library::ImportControl::new();
    let preview = db
        .with(|conn| library::preview_import(conn, &library_dir, &control))
        .unwrap();
    assert_eq!(preview.len(), 1);
    assert_eq!(preview[0].series_title, "Wrong Series");
    assert_eq!(preview[0].page_count, 3);

    let override_entry = comic_reader_lib::models::ImportOverride {
        relative_path: preview[0].relative_path.clone(),
        series_title: Some("Sonic the Hedgehog".into()),
        title: Some("Fallout".into()),
        ..Default::default()
    };
    let result = db
        .with(|conn| {
            library::commit_import(conn, &library_dir, &control, &[override_entry], |_| {})
        })
        .unwrap();
    assert_eq!(result.files_imported, 1);

    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "number".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    let issue = &snapshot.issues[0];
    assert_eq!(issue.series_title.as_deref(), Some("Sonic the Hedgehog"));
    assert_eq!(issue.title, "Fallout");

    // A second preview skips files that are already indexed.
    let preview = db
        .with(|conn| library::preview_import(conn, &library_dir, &control))
        .unwrap();
    assert!(preview.is_empty());
}

#[test]
fn variants_are_returned_for_comparison() {
    let (dir, db) = setup("variant-compare");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    write_cbz_with_info(&library_dir.join("Sonic 001.cbz"), 3, "Sonic", "1", "Issue 1");
    write_cbz_with_info(&library_dir.join("Sonic 001B.cbz"), 3, "Sonic", "1B", "Issue 1 Cover B");
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();
    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "number".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    let base = snapshot.issues.iter().find(|issue| issue.number == "1").unwrap().clone();
    let variant = snapshot.issues.iter().find(|issue| issue.number == "1B").unwrap().clone();
    db.with(|conn| {
        repo::update_issue(
            conn,
            &variant.id,
            &IssuePatch {
                variant_of: Some(base.id.clone()),
                edition: Some("Cover B".into()),
                ..Default::default()
            },
        )
    })
    .unwrap();

    let list = db.with(|conn| repo::issue_variants(conn, &base.id)).unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, base.id);
    assert_eq!(list[1].id, variant.id);

    let list = db.with(|conn| repo::issue_variants(conn, &variant.id)).unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, base.id);
}

#[test]
fn migration_from_v4_preserves_progress_and_adds_session_columns() {
    let (dir, db) = setup("migrate-v4");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    write_cbz_with_info(&library_dir.join("Sonic 001.cbz"), 4, "Sonic", "1", "Issue 1");
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();
    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "number".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    let issue = &snapshot.issues[0];
    db.with(|conn| repo::save_progress(conn, &issue.id, 2, None))
        .unwrap();

    // Simulate a v4 database: remove the v5 columns and reset the version so
    // the next open has to run MIGRATION_5.
    db.with(|conn| {
        conn.execute_batch(
            "DROP INDEX IF EXISTS idx_sessions_status;
             ALTER TABLE progress DROP COLUMN scroll_offset;
             ALTER TABLE reading_sessions DROP COLUMN last_page;
             ALTER TABLE reading_sessions DROP COLUMN status;
             ALTER TABLE reading_sessions DROP COLUMN resumed_at;
             ALTER TABLE issues DROP COLUMN metadata_locked;
             ALTER TABLE series DROP COLUMN metadata_locked;
             ALTER TABLE collections DROP COLUMN is_smart;
             ALTER TABLE collections DROP COLUMN smart_rules;
             ALTER TABLE collections DROP COLUMN sort_mode;
             DROP TABLE IF EXISTS file_bookmarks;
             DROP TABLE IF EXISTS file_progress;
             UPDATE meta SET value = '4' WHERE key = 'schema_version';",
        )?;
        Ok(())
    })
    .unwrap();
    drop(db);

    let db = Database::open(&dir.join("db")).unwrap();
    let version: String = db
        .with(|conn| {
            conn.query_row(
                "SELECT value FROM meta WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .map_err(Into::into)
        })
        .unwrap();
    assert_eq!(version, "9");

    // Existing progress survives, with the new column defaulted.
    let reader = db.with(|conn| repo::reader_payload(conn, &issue.id)).unwrap();
    assert_eq!(reader.progress.page, 2);
    assert_eq!(reader.progress.scroll_offset, 0.0);

    // The new session columns work after migration.
    let session = db
        .with(|conn| repo::start_session(conn, &issue.id, 1))
        .unwrap();
    db.with(|conn| repo::close_session(conn, &session, 1, 2)).unwrap();
    let history = db.with(|conn| repo::reading_history(conn, 10)).unwrap();
    assert_eq!(history.sessions.len(), 1);
    assert_eq!(history.sessions[0].last_page, 2);
}


#[test]
fn scaled_page_bytes_respects_requested_width_and_caches_nothing() {
    let (dir, db) = setup("page-scaled");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    write_cbz_with_info(&library_dir.join("Sonic 001.cbz"), 2, "Sonic", "1", "Issue 1");
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();
    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "number".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    let file_id = snapshot.issues[0].file_id.clone().unwrap();

    let (bytes, mime) = db
        .with(|conn| repo::page_bytes_scaled(conn, &file_id, 0, 1024))
        .unwrap();
    assert_eq!(mime, "image/jpeg");
    let image = image::load_from_memory(&bytes).unwrap();
    assert_eq!(image.width(), 1024);
    assert!(image.height() > 0);

    // Widths are clamped to a sane range rather than passed straight through.
    let (bytes, _) = db
        .with(|conn| repo::page_bytes_scaled(conn, &file_id, 0, 1))
        .unwrap();
    let image = image::load_from_memory(&bytes).unwrap();
    assert_eq!(image.width(), 320);
}

#[test]
fn fts_search_finds_notes_and_respects_spoilers() {
    let (dir, db) = setup("fts-search");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    write_cbz_with_info(&library_dir.join("Sonic 001.cbz"), 4, "Sonic", "1", "Fallout");
    write_cbz_with_info(
        &library_dir.join("Sonic 002.cbz"),
        4,
        "Sonic",
        "2",
        "The Fate of Dr. Eggman",
    );
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();
    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "number".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    let first = snapshot.issues.iter().find(|issue| issue.number == "1").unwrap();
    let second = snapshot.issues.iter().find(|issue| issue.number == "2").unwrap();

    // Bookmark notes are indexed through the trigger.
    db.with(|conn| repo::add_bookmark(conn, &first.id, 1, Some("Quiet scene"), Some("whisper")))
        .unwrap();

    // Snippets are only produced for content the reader is allowed to see, so
    // turn spoiler protection off for the snippet assertions.
    let settings = Settings {
        spoiler_protection: false,
        ..Settings::default()
    };
    db.with(|conn| repo::save_settings(conn, &settings)).unwrap();

    let hits = db
        .with(|conn| {
            repo::search_filtered(
                conn,
                &comic_reader_lib::models::SearchFilter {
                    query: "whisper".into(),
                    ..Default::default()
                },
            )
        })
        .unwrap();
    assert_eq!(hits.issues.len(), 1);
    assert_eq!(hits.issues[0].id, first.id);
    assert!(hits.snippets.contains_key(&first.id));

    // Prefix tokens match ("Eggm" -> Eggman) and the snippet carries the hit.
    let hits = db
        .with(|conn| {
            repo::search_filtered(
                conn,
                &comic_reader_lib::models::SearchFilter {
                    query: "Eggm".into(),
                    ..Default::default()
                },
            )
        })
        .unwrap();
    assert!(hits.issues.iter().any(|issue| issue.id == second.id));
    let snippet = hits.snippets.get(&second.id).cloned().unwrap_or_default();
    assert!(snippet.to_lowercase().contains("eggman"), "snippet was {snippet:?}");

    // Spoiler-protected issues never get a snippet, even when they match.
    let settings = Settings {
        spoiler_protection: true,
        ..Settings::default()
    };
    db.with(|conn| repo::save_settings(conn, &settings)).unwrap();
    let protected = db
        .with(|conn| {
            repo::search_filtered(
                conn,
                &comic_reader_lib::models::SearchFilter {
                    query: "test issue".into(),
                    ..Default::default()
                },
            )
        })
        .unwrap();
    for issue in &protected.issues {
        if issue.spoiler_protected {
            assert!(!protected.snippets.contains_key(&issue.id));
        }
    }
}

#[test]
fn fts_search_pages_with_a_keyset_cursor() {
    let (dir, db) = setup("fts-paging");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    for number in 1..=5 {
        write_cbz_with_info(
            &library_dir.join(format!("Sonic {number:03}.cbz")),
            3,
            "Sonic",
            &number.to_string(),
            &format!("Chapter {number}"),
        );
    }
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();

    let mut collected = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..10 {
        let page = db
            .with(|conn| {
                repo::search_filtered(
                    conn,
                    &comic_reader_lib::models::SearchFilter {
                        query: "Chapter".into(),
                        sort: "series".into(),
                        cursor: cursor.clone(),
                        limit: Some(2),
                        ..Default::default()
                    },
                )
            })
            .unwrap();
        assert!(page.issues.len() <= 2);
        collected.extend(page.issues.iter().map(|issue| issue.id.clone()));
        match page.next_cursor {
            Some(next) if !page.issues.is_empty() => cursor = Some(next),
            _ => break,
        }
    }
    collected.sort();
    collected.dedup();
    assert_eq!(collected.len(), 5, "keyset paging must not skip or repeat rows");
}

#[test]
fn search_records_recent_queries_and_filters() {
    let (dir, db) = setup("search-filters");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    write_cbz_with_info(&library_dir.join("Sonic 001.cbz"), 3, "Sonic", "1", "Fallout");
    write_cbz_with_info(&library_dir.join("Sonic 002.cbz"), 3, "Sonic", "2", "Fallout Again");
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();

    let all = db
        .with(|conn| {
            repo::search_filtered(
                conn,
                &comic_reader_lib::models::SearchFilter {
                    query: "Fallout".into(),
                    ..Default::default()
                },
            )
        })
        .unwrap();
    assert_eq!(all.issues.len(), 2);

    let filtered = db
        .with(|conn| {
            repo::search_filtered(
                conn,
                &comic_reader_lib::models::SearchFilter {
                    query: "Fallout".into(),
                    publisher: Some("IDW Publishing".into()),
                    year_from: Some(2018),
                    year_to: Some(2018),
                    ..Default::default()
                },
            )
        })
        .unwrap();
    assert_eq!(filtered.issues.len(), 2);

    let wrong_publisher = db
        .with(|conn| {
            repo::search_filtered(
                conn,
                &comic_reader_lib::models::SearchFilter {
                    query: "Fallout".into(),
                    publisher: Some("Marvel".into()),
                    ..Default::default()
                },
            )
        })
        .unwrap();
    assert!(wrong_publisher.issues.is_empty());

    let recent = db.with(|conn| repo::recent_searches(conn)).unwrap();
    assert_eq!(recent.first().map(|entry| entry.query.as_str()), Some("Fallout"));
    db.with(|conn| repo::clear_search_history(conn)).unwrap();
    assert!(db.with(|conn| repo::recent_searches(conn)).unwrap().is_empty());
}

#[test]
fn filter_only_search_pages_and_sorts_without_a_text_query() {
    let (dir, db) = setup("search-filter-only");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    for number in 1..=4 {
        write_cbz_with_info(
            &library_dir.join(format!("Sonic {number:03}.cbz")),
            3,
            "Sonic",
            &number.to_string(),
            &format!("Chapter {number}"),
        );
    }
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();

    let mut collected = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..10 {
        let page = db
            .with(|conn| {
                repo::search_filtered(
                    conn,
                    &comic_reader_lib::models::SearchFilter {
                        query: String::new(),
                        publisher: Some("IDW Publishing".into()),
                        cursor: cursor.clone(),
                        limit: Some(2),
                        ..Default::default()
                    },
                )
            })
            .unwrap();
        collected.extend(page.issues.iter().map(|issue| issue.id.clone()));
        match page.next_cursor {
            Some(next) if !page.issues.is_empty() => cursor = Some(next),
            _ => break,
        }
    }
    collected.sort();
    collected.dedup();
    assert_eq!(collected.len(), 4);
}

#[test]
fn metadata_lock_protects_manual_edits_from_refresh_and_bulk_edits() {
    let (dir, db) = setup("metadata-lock");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    let archive = library_dir.join("Sonic 001.cbz");
    write_cbz_with_info(&archive, 3, "Sonic the Hedgehog", "1", "Fallout");
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();

    let snapshot = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "number".into(),
                    descending: false,
                },
            )
        })
        .unwrap();
    let issue_id = snapshot.issues[0].id.clone();
    assert!(!snapshot.issues[0].metadata_locked);
    assert!(!snapshot.series[0].metadata_locked);

    let manual = IssuePatch {
        description: Some("My own synopsis".into()),
        ..Default::default()
    };
    let updated = db
        .with(|conn| repo::update_issue(conn, &issue_id, &manual))
        .unwrap();
    assert!(updated.metadata_locked);
    // Read the raw column: `repo::issue` applies spoiler filtering, which nulls
    // the synopsis of an unread issue.
    let stored: Option<String> = db
        .with(|conn| {
            conn.query_row(
                "SELECT description FROM issues WHERE id = ?1",
                [&issue_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
        })
        .unwrap();
    assert_eq!(stored.as_deref(), Some("My own synopsis"));

    // An automatic refresh must not clobber a locked issue.
    let refreshed = db
        .with(|conn| repo::refresh_issue_metadata(conn, &issue_id, false))
        .unwrap();
    assert!(!refreshed);
    let after: Option<String> = db
        .with(|conn| {
            conn.query_row(
                "SELECT description FROM issues WHERE id = ?1",
                [&issue_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
        })
        .unwrap();
    assert_eq!(after.as_deref(), Some("My own synopsis"));

    // Bulk edits skip locked issues unless they are changing the lock itself.
    let bulk = IssuePatch {
        writer: Some("Someone Else".into()),
        ..Default::default()
    };
    let changed = db
        .with(|conn| repo::bulk_update(conn, std::slice::from_ref(&issue_id), &bulk))
        .unwrap();
    assert!(changed.is_empty());
    let after = db.with(|conn| repo::issue(conn, &issue_id)).unwrap();
    assert_eq!(after.writer.as_deref(), Some("Ian Flynn"));

    // Unlocking allows an explicit refresh to restore embedded metadata.
    let unlock = IssuePatch {
        metadata_locked: Some(false),
        ..Default::default()
    };
    let unlocked = db
        .with(|conn| repo::update_issue(conn, &issue_id, &unlock))
        .unwrap();
    assert!(!unlocked.metadata_locked);
    let refreshed = db
        .with(|conn| repo::refresh_issue_metadata(conn, &issue_id, true))
        .unwrap();
    assert!(refreshed);
    let after: Option<String> = db
        .with(|conn| {
            conn.query_row(
                "SELECT description FROM issues WHERE id = ?1",
                [&issue_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
        })
        .unwrap();
    assert_eq!(after.as_deref(), Some("A test issue for the reader."));

    // Series edits lock the series too, and locked issues keep their universe.
    let series_id = snapshot.series[0].id.clone();
    let patch = comic_reader_lib::models::SeriesPatch {
        universe: Some("Sonic".into()),
        ..Default::default()
    };
    let detail = db
        .with(|conn| repo::update_series(conn, &series_id, &patch))
        .unwrap();
    assert!(detail.summary.metadata_locked);
    assert_eq!(detail.summary.universe.as_deref(), Some("Sonic"));
}

#[test]
fn smart_collections_compile_typed_rules_and_reject_injection() {
    use comic_reader_lib::models::{CollectionPatch, SmartRule, SmartRules};

    let (dir, db) = setup("smart-collections");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    write_cbz_with_info(
        &library_dir.join("Sonic 001.cbz"),
        3,
        "Sonic the Hedgehog",
        "1",
        "Fallout",
    );
    write_cbz_with_info(
        &library_dir.join("Sonic 002.cbz"),
        3,
        "Sonic the Hedgehog",
        "2",
        "The Fate of Dr. Eggman",
    );
    db.with(|conn| library::import_root(conn, &library_dir)).unwrap();

    let rules = SmartRules {
        all: vec![SmartRule {
            field: "series".into(),
            op: "contains".into(),
            value: "sonic".into(),
        }],
        any: Vec::new(),
    };
    let collection = db
        .with(|conn| {
            repo::create_smart_collection(conn, "All Sonic", Some("rule test"), &rules, Some("series"))
        })
        .unwrap();
    assert!(collection.is_smart);
    assert_eq!(collection.item_count, 2);

    let detail = db.with(|conn| repo::collection_detail(conn, &collection.id)).unwrap();
    assert_eq!(detail.items.len(), 2);
    // Sorted by series then number.
    assert_eq!(detail.items[0].number, "1");
    assert_eq!(detail.items[1].number, "2");

    // `all` and `any` groups are combined with AND: narrowing by number keeps
    // only #2 even though the `any` group would also admit it.
    let combined = SmartRules {
        all: vec![SmartRule {
            field: "series".into(),
            op: "contains".into(),
            value: "sonic".into(),
        }],
        any: vec![SmartRule {
            field: "number".into(),
            op: "eq".into(),
            value: "2".into(),
        }],
    };
    db.with(|conn| {
        repo::update_collection(
            conn,
            &collection.id,
            &CollectionPatch {
                smart_rules: Some(combined),
                ..Default::default()
            },
        )
    })
    .unwrap();
    let detail = db.with(|conn| repo::collection_detail(conn, &collection.id)).unwrap();
    assert_eq!(detail.items.len(), 1);
    assert_eq!(detail.items[0].number, "2");

    // Rules track the catalog instead of a stored item list.
    let narrowed = SmartRules {
        all: vec![SmartRule {
            field: "title".into(),
            op: "contains".into(),
            value: "eggman".into(),
        }],
        any: Vec::new(),
    };
    db.with(|conn| {
        repo::update_collection(
            conn,
            &collection.id,
            &CollectionPatch {
                smart_rules: Some(narrowed),
                ..Default::default()
            },
        )
    })
    .unwrap();
    let detail = db.with(|conn| repo::collection_detail(conn, &collection.id)).unwrap();
    assert_eq!(detail.items.len(), 1);
    assert_eq!(detail.items[0].number, "2");

    // Arbitrary fields and operators never reach SQL.
    let injection = SmartRules {
        all: vec![SmartRule {
            field: "title = title OR 1=1; --".into(),
            op: "eq".into(),
            value: "x".into(),
        }],
        any: Vec::new(),
    };
    let result = db.with(|conn| {
        repo::create_smart_collection(conn, "Injection", None, &injection, None)
    });
    assert!(result.is_err());

    let bad_op = SmartRules {
        all: vec![SmartRule {
            field: "title".into(),
            op: "union select".into(),
            value: "x".into(),
        }],
        any: Vec::new(),
    };
    assert!(db
        .with(|conn| repo::create_smart_collection(conn, "Injection", None, &bad_op, None))
        .is_err());

    // Empty rules match nothing rather than the whole library.
    let empty = db
        .with(|conn| {
            repo::create_smart_collection(conn, "Empty", None, &SmartRules::default(), None)
        })
        .unwrap();
    assert_eq!(empty.item_count, 0);
    let detail = db.with(|conn| repo::collection_detail(conn, &empty.id)).unwrap();
    assert!(detail.items.is_empty());

    // Search's collection filter understands smart collections too.
    let results = db
        .with(|conn| {
            repo::search_filtered(
                conn,
                &comic_reader_lib::models::SearchFilter {
                    query: String::new(),
                    publisher: None,
                    series_id: None,
                    universe: None,
                    status: None,
                    format: None,
                    year_from: None,
                    year_to: None,
                    collection_id: Some(collection.id.clone()),
                    sort: "series".into(),
                    cursor: None,
                    limit: Some(20),
                },
            )
        })
        .unwrap();
    assert_eq!(results.issues.len(), 1);
    assert_eq!(results.issues[0].number, "2");
}

#[test]
fn unmatched_files_stay_readable_and_match_without_losing_progress() {
    use comic_reader_lib::models::{ImportOverride, MatchFilePatch};

    let (dir, db) = setup("unmatched");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    write_cbz_with_info(&library_dir.join("Sonic 001.cbz"), 4, "Sonic", "1", "Issue 1");
    write_cbz(&library_dir.join("Loose Scan.cbz"), 6, false);

    // Import the Sonic issue normally and keep the other file unmatched.
    let overrides = vec![ImportOverride {
        relative_path: "Loose Scan.cbz".into(),
        keep_unmatched: Some(true),
        ..Default::default()
    }];
    let result = db
        .with(|conn| library::commit_import(conn, &library_dir, &library::ImportControl::new(), &overrides, |_| {}))
        .unwrap();
    assert_eq!(result.files_imported, 2);
    assert_eq!(result.issues_created, 1);

    let unmatched = db.with(|conn| repo::unmatched_files(conn)).unwrap();
    assert_eq!(unmatched.len(), 1);
    let file = &unmatched[0];
    assert_eq!(file.relative_path, "Loose Scan.cbz");
    assert_eq!(file.page_count, 6);
    assert_eq!(file.suggested_series, "Loose Scan");
    assert_eq!(file.progress_status, "unread");

    // It reads through the file-scoped reader path and keeps its own progress.
    let reader = db.with(|conn| repo::file_reader_payload(conn, &file.id)).unwrap();
    assert!(reader.issue_id.is_none());
    assert_eq!(reader.pages.len(), 6);
    let progress = db
        .with(|conn| repo::save_file_progress(conn, &file.id, 3, Some(0.25)))
        .unwrap();
    assert_eq!(progress.page, 3);
    assert_eq!(progress.status, "in-progress");
    assert_eq!(progress.scroll_offset, 0.25);
    let bookmark = db
        .with(|conn| repo::add_file_bookmark(conn, &file.id, 3, Some("Mark"), Some("note")))
        .unwrap();
    assert!(bookmark.get("id").is_some());

    // Matching creates the issue but reuses the same file row and moves the
    // progress/bookmark from file scope to the issue.
    let patch = MatchFilePatch {
        series_title: Some("Loose Scan".into()),
        number: Some("7".into()),
        title: Some("Loose Scan #7".into()),
        ..Default::default()
    };
    let issue = db
        .with(|conn| repo::match_file_to_new_issue(conn, &file.id, &patch))
        .unwrap();
    assert_eq!(issue.file_id.as_deref(), Some(file.id.as_str()));
    assert_eq!(issue.number, "7");
    assert_eq!(issue.page_count, 6);
    assert_eq!(issue.last_page, 3);
    assert_eq!(issue.status, "in-progress");
    assert!(db.with(|conn| repo::unmatched_files(conn)).unwrap().is_empty());
    let marks = db.with(|conn| repo::bookmarks(conn, &issue.id)).unwrap();
    assert_eq!(marks.len(), 1);
    let file_marks = db.with(|conn| repo::file_bookmarks(conn, &file.id)).unwrap();
    assert!(file_marks.is_empty());
    let file_progress = db.with(|conn| repo::file_progress(conn, &file.id)).unwrap();
    assert_eq!(file_progress.page, 0);

    // A second match attempt on the same file is rejected.
    let again = db.with(|conn| repo::match_file_to_new_issue(conn, &file.id, &patch));
    assert!(again.is_err());

    // Matching to an existing fileless issue moves the file record too.
    write_cbz(&library_dir.join("Loose Annual.cbz"), 3, false);
    let overrides = vec![ImportOverride {
        relative_path: "Loose Annual.cbz".into(),
        keep_unmatched: Some(true),
        ..Default::default()
    }];
    db.with(|conn| library::commit_import(conn, &library_dir, &library::ImportControl::new(), &overrides, |_| {}))
        .unwrap();
    let unmatched = db.with(|conn| repo::unmatched_files(conn)).unwrap();
    assert_eq!(unmatched.len(), 1);
    let annual = &unmatched[0];
    db.with(|conn| repo::save_file_progress(conn, &annual.id, 2, None))
        .unwrap();
    let fileless = db.with(|conn| repo::fileless_issues(conn)).unwrap();
    assert_eq!(fileless.len(), 0);

    // A fileless issue can adopt the annual, preserving its file progress.
    let series = db
        .with(|conn| repo::create_series(conn, "Annuals", None, None))
        .unwrap();
    db.with(|conn| {
            conn.execute(
                "INSERT INTO issues (id, file_id, series_id, title, number, sort_number,
                    created_at, updated_at)
                 VALUES ('fileless-annual', NULL, ?1, 'Annual 2020', '2020', 2020,
                    ?2, ?2)",
                rusqlite::params![series.summary.id, "2026-01-01T00:00:00Z"],
            )?;
            Ok(())
        })
        .unwrap();
    assert_eq!(db.with(|conn| repo::fileless_issues(conn)).unwrap().len(), 1);
    let matched = db
        .with(|conn| repo::match_file_to_existing_issue(conn, &annual.id, "fileless-annual"))
        .unwrap();
    assert_eq!(matched.file_id.as_deref(), Some(annual.id.as_str()));
    assert_eq!(matched.last_page, 2);
    assert_eq!(matched.series_title.as_deref(), Some("Annuals"));
    assert!(db.with(|conn| repo::unmatched_files(conn)).unwrap().is_empty());
    assert!(db.with(|conn| repo::fileless_issues(conn)).unwrap().is_empty());
}
