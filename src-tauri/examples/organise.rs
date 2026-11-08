use std::path::PathBuf;

use comic_reader_lib::db::Database;
use comic_reader_lib::models::LibraryFilter;
use comic_reader_lib::repo;

fn main() {
    let db_dir = std::env::args().nth(1).map(PathBuf::from).expect("db dir");
    let db = Database::open(&db_dir).expect("open db");
    let filter = LibraryFilter {
        search: String::new(),
        universe: None,
        publisher: None,
        series_id: None,
        status: None,
        sort: "number".into(),
        descending: false,
    };
    let snapshot = db.with(|conn| repo::library_snapshot(conn, &filter)).unwrap();
    if snapshot.issues.is_empty() {
        println!("no issues");
        return;
    }
    let existing_orders = db.with(|conn| repo::orders(conn)).unwrap();
    if existing_orders.is_empty() {
        let order = db.with(|conn| repo::build_publication_order(conn, "Sonic — publication order")).unwrap();
        let mut ids: Vec<String> = order.items.iter().map(|item| item.issue_id.clone()).collect();
        if ids.len() > 2 {
            ids.swap(0, 1);
            db.with(|conn| repo::reorder_order(conn, &order.order.id, &ids)).unwrap();
        }
        db.with(|conn| repo::create_order(conn, "Sonic — story chronological", Some("Hand-curated in-universe chronology"), "chronological")).unwrap();
        println!("created reading orders");
    }
    let existing_collections = db.with(|conn| repo::collections(conn)).unwrap();
    if existing_collections.is_empty() {
        let favourites = db.with(|conn| repo::create_collection(conn, "Best Shadow appearances", Some("Shadow-centric issues worth rereading"))).unwrap();
        let shadow: Vec<String> = snapshot
            .issues
            .iter()
            .filter(|issue| issue.series_title.as_deref() == Some("Shadow the Hedgehog"))
            .take(3)
            .map(|issue| issue.id.clone())
            .collect();
        db.with(|conn| repo::add_to_collection(conn, &favourites.id, &shadow)).unwrap();

        let pile = db.with(|conn| repo::create_collection(conn, "Read next", Some("A short queue for the weekend"))).unwrap();
        let next: Vec<String> = snapshot.issues.iter().take(2).map(|issue| issue.id.clone()).collect();
        db.with(|conn| repo::add_to_collection(conn, &pile.id, &next)).unwrap();
        println!("created collections");
    }

    // Simulate some reading history so stats and progress states are populated.
    for (index, issue) in snapshot.issues.iter().enumerate() {
        if index % 5 == 0 {
            let _ = db.with(|conn| repo::save_progress(conn, &issue.id, 7, Some(0.5)));
        } else if index % 4 == 0 {
            let _ = db.with(|conn| repo::save_progress(conn, &issue.id, 3, None));
        }
    }
    println!("organised library");
}
