use std::path::PathBuf;

use comic_reader_lib::db::Database;
use comic_reader_lib::repo;

/// Writes the page images for every readable issue so the browser mock harness
/// (`?mock`) can serve them from `/mock-pages/{fileId}/{index}.{ext}`.
///
/// Usage, from `src-tauri`:
/// `cargo run --example export-pages -- ~/.local/share/com.comicreader.app ../public/mock-pages`
fn main() {
    let db_dir = std::env::args().nth(1).map(PathBuf::from).expect("db dir");
    let out_dir = std::env::args()
        .nth(2)
        .map(PathBuf::from)
        .expect("output dir");

    let db = Database::open(&db_dir).expect("open database");
    let issues = db
        .with(|conn| {
            repo::library_snapshot(
                conn,
                &comic_reader_lib::models::LibraryFilter {
                    search: String::new(),
                    universe: None,
                    publisher: None,
                    series_id: None,
                    status: None,
                    sort: "recent".into(),
                    descending: false,
                },
            )
        })
        .expect("read snapshot")
        .issues;

    let mut written = 0usize;
    let mut skipped = 0usize;
    for issue in &issues {
        let Some(file_id) = issue.file_id.as_ref() else {
            continue;
        };
        let Some(payload) = db
            .with(|conn| repo::reader_payload(conn, &issue.id))
            .ok()
        else {
            skipped += 1;
            continue;
        };

        let dir = out_dir.join(file_id);
        std::fs::create_dir_all(&dir).expect("create page dir");
        for (index, page) in payload.pages.iter().enumerate() {
            let ext = page
                .name
                .rsplit('.')
                .next()
                .filter(|value| !value.is_empty())
                .unwrap_or("jpg")
                .to_ascii_lowercase();
            let target = dir.join(format!("{index}.{ext}"));
            match db.with(|conn| repo::page_bytes(conn, file_id, index as i64)) {
                Ok((bytes, _mime)) => {
                    std::fs::write(&target, bytes).expect("write page");
                    written += 1;
                }
                Err(err) => {
                    eprintln!("skip {} page {}: {err}", issue.title, index);
                    skipped += 1;
                }
            }
        }
    }

    println!(
        "exported {written} page images to {} ({skipped} skipped)",
        out_dir.display()
    );
}
