use std::path::PathBuf;

use comic_reader_lib::db::Database;
use comic_reader_lib::library;

fn main() {
    let mut args = std::env::args().skip(1);
    let db_dir = args
        .next()
        .map(PathBuf::from)
        .expect("usage: seed <db-dir> <library-dir>");
    let library_dir = args
        .next()
        .map(PathBuf::from)
        .expect("usage: seed <db-dir> <library-dir>");
    let db = Database::open(&db_dir).expect("open database");
    let result = db
        .with(|conn| library::import_root(conn, &library_dir))
        .expect("import");
    println!(
        "imported {} files, {} issues, {} series, {} warnings",
        result.files_imported,
        result.issues_created,
        result.series_created,
        result.warnings.len()
    );
    for warning in result.warnings {
        println!("warning: {} — {}", warning.path, warning.message);
    }
}
