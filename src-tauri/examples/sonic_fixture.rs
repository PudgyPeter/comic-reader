//! Generates a synthetic, copyright-free "Sonic"-shaped test library.
//!
//! It writes real CBZ archives (PNG pages + ComicInfo.xml) so the importer,
//! gallery, reader, missing-issue detection and variant grouping all get
//! exercised with realistic structure: numbering gaps, an annual, a variant
//! cover, and a mix of read states.
//!
//! usage: cargo run --example sonic_fixture -- <library-dir> <db-dir>

use std::io::Write;
use std::path::{Path, PathBuf};

use image::{ImageFormat, Rgb, RgbImage};
use zip::write::SimpleFileOptions;

use comic_reader_lib::db::Database;
use comic_reader_lib::{library, repo};

fn main() {
    let mut args = std::env::args().skip(1);
    let library = args.next().map(PathBuf::from).expect("usage: <library-dir> <db-dir>");
    let db_dir = args.next().map(PathBuf::from).expect("usage: <library-dir> <db-dir>");

    std::fs::create_dir_all(&library).expect("create library dir");
    println!("writing synthetic pages to {}", library.display());

    let mut written = 0usize;
    for spec in specs() {
        write_issue(&library, &spec);
        written += 1;
    }
    println!("wrote {written} issues");

    let db = Database::open(&db_dir).expect("open database");
    let result = db
        .with(|conn| library::import_root(conn, &library))
        .expect("import");
    println!(
        "imported {} files ({} skipped), {} issues, {} series, {} warnings",
        result.files_imported,
        result.files_skipped,
        result.issues_created,
        result.series_created,
        result.warnings.len()
    );
    for warning in &result.warnings {
        println!("warning: {} — {}", warning.path, warning.message);
    }

    seed_state(&db);
    println!("\ndone. Open the app (or npm run dev + ?mock#/) to browse the fixture.");
}

struct Spec {
    file: String,
    series: String,
    number: String,
    title: String,
    year: String,
    pages: u32,
    hue: u8,
    annual: bool,
}

fn specs() -> Vec<Spec> {
    let mut out = Vec::new();
    // Main run: 1..20 with deliberate gaps at 7, 12, 15.
    let titles = [
        "Fallout",
        "The Fate of Dr. Eggman",
        "Battle for Angel Island",
        "The Last Minute",
        "The Grand Tour",
        "The Bonds of Friendship",
        "Tangle and Whisper",
        "Escape from the Egg Fleet",
        "Test Run",
        "Metal Virus",
        "Turbulence",
        "Infection",
        "The Last Tangle",
        "Overpowered",
        "Out of the Blue",
    ];
    let mut title_index = 0usize;
    for number in 1..=20u32 {
        if [7, 12, 15].contains(&number) {
            continue;
        }
        out.push(Spec {
            file: format!("Sonic the Hedgehog {number:03}.cbz"),
            series: "Sonic the Hedgehog".into(),
            number: number.to_string(),
            title: titles[title_index % titles.len()].to_string(),
            year: "2018".into(),
            pages: 6,
            hue: (number * 13 % 255) as u8,
            annual: false,
        });
        title_index += 1;
    }

    // Variant cover for issue 3: same base issue, alternate cover.
    out.push(Spec {
        file: "Sonic the Hedgehog 003 (Cover B).cbz".into(),
        series: "Sonic the Hedgehog".into(),
        number: "3".into(),
        title: "Battle for Angel Island".into(),
        year: "2018".into(),
        pages: 6,
        hue: 200,
        annual: false,
    });

    // Annual: non-numeric, must not create a numbering gap.
    out.push(Spec {
        file: "Sonic the Hedgehog Annual 2020.cbz".into(),
        series: "Sonic the Hedgehog".into(),
        number: "Annual 2020".into(),
        title: "Annual 2020".into(),
        year: "2020".into(),
        pages: 5,
        hue: 90,
        annual: true,
    });

    // Companion run with a single gap.
    for number in 1..=8u32 {
        if number == 4 {
            continue;
        }
        out.push(Spec {
            file: format!("Sonic Universe {number:03}.cbz"),
            series: "Sonic Universe".into(),
            number: number.to_string(),
            title: "Sonic Universe arc".into(),
            year: "2019".into(),
            pages: 4,
            hue: (number * 31 % 255) as u8,
            annual: false,
        });
    }
    out
}

fn write_issue(dir: &Path, spec: &Spec) {
    let path = dir.join(&spec.file);
    let file = std::fs::File::create(&path).expect("create cbz");
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    for page in 0..spec.pages {
        let name = format!("{page:03}.png");
        zip.start_file(&name, options).expect("start entry");
        let png = page_png(page, spec.pages, spec.hue);
        zip.write_all(&png).expect("write page");
    }

    zip.start_file("ComicInfo.xml", options).expect("start comicinfo");
    let xml = comic_info(spec);
    zip.write_all(xml.as_bytes()).expect("write comicinfo");
    zip.finish().expect("finish zip");
}

fn comic_info(spec: &Spec) -> String {
    let summary = if spec.annual {
        "Synthetic annual used for app testing. Not the published issue."
    } else {
        "Synthetic test page generated for the comic-reader app. Not the published issue."
    };
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<ComicInfo>
  <Series>{series}</Series>
  <Number>{number}</Number>
  <Title>{title}</Title>
  <Publisher>IDW Publishing</Publisher>
  <Year>{year}</Year>
  <Writer>Test Writer</Writer>
  <Penciller>Test Artist</Penciller>
  <CoverArtist>Test Cover Artist</CoverArtist>
  <Summary>{summary}</Summary>
</ComicInfo>
"#,
        series = spec.series,
        number = spec.number,
        title = spec.title,
        year = spec.year,
        summary = summary,
    )
}

/// A page with a distinct colour plus one white bar per page number, so page
/// order is visually obvious without embedding a font.
fn page_png(page: u32, total: u32, hue: u8) -> Vec<u8> {
    let width = 700u32;
    let height = 1000u32;
    let mut image = RgbImage::new(width, height);
    let bg = Rgb([hue, (hue as u32 * 3 % 200) as u8, 200u8.saturating_sub(hue / 3)]);
    for pixel in image.pixels_mut() {
        *pixel = bg;
    }
    for bar in 0..=page {
        let x0 = 40 + bar * 26;
        if x0 + 16 >= width - 40 {
            break;
        }
        for y in 40..140 {
            for x in x0..x0 + 16 {
                image.put_pixel(x, y, Rgb([255, 255, 255]));
            }
        }
    }
    // Footer ticks encode the total page count on the last page.
    if page + 1 == total {
        for x in 40..width - 40 {
            for y in height - 80..height - 60 {
                image.put_pixel(x, y, Rgb([255, 255, 255]));
            }
        }
    }
    let mut buffer = std::io::Cursor::new(Vec::new());
    image.write_to(&mut buffer, ImageFormat::Png).expect("encode png");
    buffer.into_inner()
}

/// Seeds realistic reader state so every screen has data on first launch:
/// a partly-read series, a completed issue, bookmarks, a collection, an order,
/// and a spy on the variant grouping.
fn seed_state(db: &Database) {
    use comic_reader_lib::models::{IssuePatch, LibraryFilter};

    let filter = LibraryFilter {
        search: String::new(),
        universe: None,
        publisher: None,
        series_id: None,
        status: None,
        sort: "number".into(),
        descending: false,
    };
    let snapshot = db.with(|conn| repo::library_snapshot(conn, &filter)).expect("snapshot");
    let by_title = |series: &str, number: &str| {
        snapshot
            .issues
            .iter()
            .find(|issue| {
                issue.number == number && issue.series_title.as_deref() == Some(series)
            })
            .map(|issue| issue.id.clone())
    };

    // Read #1 fully, #2 halfway.
    if let Some(id) = by_title("Sonic the Hedgehog", "1") {
        let _ = db.with(|conn| repo::save_progress(conn, &id, 5, Some(0.8)));
        let _ = db.with(|conn| repo::record_session(conn, &id, 6, 420));
    }
    if let Some(id) = by_title("Sonic the Hedgehog", "2") {
        let _ = db.with(|conn| repo::save_progress(conn, &id, 2, Some(0.8)));
        let _ = db.with(|conn| repo::record_session(conn, &id, 3, 180));
    }
    if let Some(id) = by_title("Sonic the Hedgehog", "3") {
        let _ = db.with(|conn| repo::add_bookmark(conn, &id, 1, Some("Great spread"), Some("Check the background art here.")));
    }
    if let Some(id) = by_title("Sonic the Hedgehog", "5") {
        let _ = db.with(|conn| repo::save_progress(conn, &id, 1, Some(0.8)));
    }

    // Link the Cover B file to the base issue so variant grouping is exercised.
    let base = snapshot
        .issues
        .iter()
        .find(|issue| {
            issue.number == "3"
                && issue.series_title.as_deref() == Some("Sonic the Hedgehog")
                && issue
                    .file_path
                    .as_deref()
                    .is_some_and(|path| !path.contains("Cover B"))
        })
        .map(|issue| issue.id.clone());
    let variant = snapshot
        .issues
        .iter()
        .find(|issue| {
            issue.series_title.as_deref() == Some("Sonic the Hedgehog")
                && issue
                    .file_path
                    .as_deref()
                    .is_some_and(|path| path.contains("Cover B"))
        })
        .map(|issue| issue.id.clone());
    if let (Some(base), Some(variant)) = (base, variant) {
        let patch = IssuePatch {
            variant_of: Some(base),
            edition: Some("Cover B".into()),
            ..Default::default()
        };
        match db.with(|conn| repo::update_issue(conn, &variant, &patch)) {
            Ok(_) => println!("linked Cover B to base issue #3"),
            Err(err) => println!("variant link failed: {err}"),
        }
    }
    else {
        println!("variant link skipped: base or variant issue not found");
    }

    // Mark the annual as a special edition so the edition chip renders.
    if let Some(id) = snapshot
        .issues
        .iter()
        .find(|issue| issue.number == "Annual 2020")
        .map(|issue| issue.id.clone())
    {
        let patch = IssuePatch {
            edition: Some("Annual".into()),
            ..Default::default()
        };
        if let Err(err) = db.with(|conn| repo::update_issue(conn, &id, &patch)) {
            println!("annual edition update failed: {err}");
        }
    }

    // A curated collection of the first six issues.
    let no_collections = db
        .with(|conn| repo::collections(conn))
        .map(|list| list.is_empty())
        .unwrap_or(false);
    if no_collections
        && let Ok(collection) = db.with(|conn| {
            repo::create_collection(
                conn,
                "Best of the Metal Virus",
                Some("Test collection generated with the fixture"),
            )
        })
    {
        let ids: Vec<String> = snapshot
            .issues
            .iter()
            .filter(|issue| {
                issue.series_title.as_deref() == Some("Sonic the Hedgehog")
                    && issue.number.parse::<i64>().is_ok_and(|n| n <= 6)
            })
            .map(|issue| issue.id.clone())
            .collect();
        let _ = db.with(|conn| repo::add_to_collection(conn, &collection.id, &ids));
    }

    // A generated publication order so the reader can follow one.
    if db.with(|conn| repo::orders(conn)).map(|list| list.is_empty()).unwrap_or(false) {
        let _ = db.with(|conn| repo::build_publication_order(conn, "Sonic — publication order"));
    }
}
