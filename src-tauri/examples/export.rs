use std::path::PathBuf;

use comic_reader_lib::db::Database;
use comic_reader_lib::models::LibraryFilter;
use comic_reader_lib::repo;

fn main() {
    let db_dir = std::env::args().nth(1).map(PathBuf::from).expect("db dir");
    let out = std::env::args().nth(2).map(PathBuf::from).expect("out json");
    let db = Database::open(&db_dir).expect("open database");

    let filter = LibraryFilter {
        search: String::new(),
        universe: None,
        publisher: None,
        series_id: None,
        status: None,
        sort: "recent".into(),
        descending: false,
    };
    let snapshot = db.with(|conn| repo::library_snapshot(conn, &filter)).unwrap();
    let settings = db.with(|conn| repo::settings(conn)).unwrap();
    let stats = db.with(|conn| repo::stats(conn)).unwrap();
    let orders = db.with(|conn| repo::orders(conn)).unwrap();
    let collections = db.with(|conn| repo::collections(conn)).unwrap();
    let search = db.with(|conn| repo::search(conn, "Sonic", 100)).unwrap();
    let unmatched = db.with(|conn| repo::unmatched_files(conn)).unwrap_or_default();

    let mut series_details = serde_json::Map::new();
    for series in &snapshot.series {
        let detail = db.with(|conn| repo::series_detail(conn, &series.id)).unwrap();
        series_details.insert(series.id.clone(), serde_json::to_value(detail).unwrap());
    }

    let mut order_details = serde_json::Map::new();
    for order in &orders {
        let detail = db.with(|conn| repo::order_detail(conn, &order.id)).unwrap();
        order_details.insert(order.id.clone(), serde_json::to_value(detail).unwrap());
    }

    let mut collection_details = serde_json::Map::new();
    for collection in &collections {
        let detail = db.with(|conn| repo::collection_detail(conn, &collection.id)).unwrap();
        collection_details.insert(collection.id.clone(), serde_json::to_value(detail).unwrap());
    }

    let mut readers = serde_json::Map::new();
    for issue in &snapshot.issues {
        if issue.file_id.is_some()
            && let Ok(payload) = db.with(|conn| repo::reader_payload(conn, &issue.id)) {
                readers.insert(issue.id.clone(), serde_json::to_value(payload).unwrap());
            }
    }

    let history = db.with(|conn| repo::reading_history(conn, 150)).unwrap_or_else(|_| {
        comic_reader_lib::models::HistoryPayload {
            sessions: Vec::new(),
            interrupted: Vec::new(),
            recent: Vec::new(),
        }
    });

    let payload = serde_json::json!({
        "library": snapshot,
        "history": history,
        "settings": settings,
        "stats": stats,
        "orders": orders,
        "orderDetails": order_details,
        "collections": collections,
        "collectionDetails": collection_details,
        "seriesDetails": series_details,
        "readers": readers,
        "search": search,
        "unmatchedFiles": unmatched,
        "recentSearches": db
            .with(|conn| repo::recent_searches(conn))
            .unwrap_or_default(),
    });
    std::fs::write(&out, serde_json::to_string(&payload).unwrap()).unwrap();
    println!("exported {}", out.display());
}
