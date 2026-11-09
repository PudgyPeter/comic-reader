use std::path::PathBuf;
use std::sync::Arc;

use tauri::{Emitter, State};

use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::library;
use crate::models::*;
use crate::repo;

pub struct AppState {
    pub db: Arc<Database>,
    pub cache_dir: PathBuf,
    pub import_control: Arc<library::ImportControl>,
}

#[tauri::command]
pub async fn add_library_root(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> AppResult<ImportResult> {
    let db = state.db.clone();
    let control = state.import_control.clone();
    control.reset();
    let _ = &state.cache_dir;
    tauri::async_runtime::spawn_blocking(move || {
        let target = PathBuf::from(&path);
        let root = library::normalize_root(&target)?;
        db.with(|conn| {
            library::import_root_with_progress(conn, &root, &control, |progress| {
                let _ = app.emit("import-progress", progress);
            })
        })
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn cancel_import(state: State<'_, AppState>) -> AppResult<bool> {
    state.import_control.cancel();
    Ok(true)
}

#[tauri::command]
pub async fn rescan_library(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> AppResult<Vec<ImportResult>> {
    let db = state.db.clone();
    let control = state.import_control.clone();
    control.reset();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| library::rescan_with_progress(conn, &control, |progress| {
            let _ = app.emit("import-progress", progress);
        }))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn remove_library_root(state: State<'_, AppState>, root_id: String) -> AppResult<()> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| library::remove_root(conn, &root_id))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn library_roots(state: State<'_, AppState>) -> AppResult<Vec<LibraryRoot>> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || db.with(|conn| library::read_roots(conn)))
        .await
        .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn library(state: State<'_, AppState>, filter: Option<LibraryFilter>) -> AppResult<LibrarySnapshot> {
    let db = state.db.clone();
    let filter = filter.unwrap_or(LibraryFilter {
        search: String::new(),
        universe: None,
        publisher: None,
        series_id: None,
        status: None,
        sort: "recent".into(),
        descending: false,
    });
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::library_snapshot(conn, &filter))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn filter_options(state: State<'_, AppState>) -> AppResult<serde_json::Value> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || db.with(|conn| repo::filter_options(conn)))
        .await
        .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn series_detail(state: State<'_, AppState>, series_id: String) -> AppResult<SeriesDetail> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::series_detail(conn, &series_id))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn create_series(
    state: State<'_, AppState>,
    title: String,
    publisher: Option<String>,
    universe: Option<String>,
) -> AppResult<SeriesDetail> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::create_series(conn, &title, publisher.as_deref(), universe.as_deref()))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn update_series(
    state: State<'_, AppState>,
    series_id: String,
    patch: SeriesPatch,
) -> AppResult<SeriesDetail> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::update_series(conn, &series_id, &patch))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn delete_series(state: State<'_, AppState>, series_id: String) -> AppResult<()> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || db.with(|conn| repo::delete_series(conn, &series_id)))
        .await
        .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn issue_detail(state: State<'_, AppState>, issue_id: String) -> AppResult<IssueCard> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || db.with(|conn| repo::issue(conn, &issue_id)))
        .await
        .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn update_issue(
    state: State<'_, AppState>,
    issue_id: String,
    patch: IssuePatch,
) -> AppResult<IssueCard> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::update_issue(conn, &issue_id, &patch))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn bulk_update_issues(
    state: State<'_, AppState>,
    issue_ids: Vec<String>,
    patch: IssuePatch,
) -> AppResult<Vec<IssueCard>> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::bulk_update(conn, &issue_ids, &patch))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn set_missing_reason(
    state: State<'_, AppState>,
    series_id: String,
    number: String,
    reason: Option<String>,
) -> AppResult<()> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::set_missing_reason(conn, &series_id, &number, reason.as_deref()))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn delete_issue(state: State<'_, AppState>, issue_id: String) -> AppResult<()> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || db.with(|conn| repo::delete_issue(conn, &issue_id)))
        .await
        .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn reader_payload(state: State<'_, AppState>, issue_id: String) -> AppResult<ReaderPayload> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::reader_payload(conn, &issue_id))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn set_spread_offset(
    state: State<'_, AppState>,
    issue_id: String,
    offset: i64,
) -> AppResult<i64> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::set_spread_offset(conn, &issue_id, offset))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn set_direction_override(
    state: State<'_, AppState>,
    issue_id: String,
    direction: Option<String>,
) -> AppResult<Option<String>> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::set_direction_override(conn, &issue_id, direction.as_deref()))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn save_progress(
    state: State<'_, AppState>,
    issue_id: String,
    page: i64,
    completion_threshold: Option<f64>,
) -> AppResult<ProgressState> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::save_progress(conn, &issue_id, page, completion_threshold))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn set_series_direction(
    state: State<'_, AppState>,
    series_id: String,
    direction: Option<String>,
) -> AppResult<i64> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::set_series_direction(conn, &series_id, direction.as_deref()))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn set_series_status(
    state: State<'_, AppState>,
    series_id: String,
    status: String,
) -> AppResult<i64> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::set_series_status(conn, &series_id, &status))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn set_issue_status(
    state: State<'_, AppState>,
    issue_id: String,
    status: String,
) -> AppResult<IssueCard> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::set_status(conn, &issue_id, &status))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn record_reading_session(
    state: State<'_, AppState>,
    issue_id: String,
    pages: i64,
    seconds: i64,
) -> AppResult<()> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::record_session(conn, &issue_id, pages, seconds))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn search_library(
    state: State<'_, AppState>,
    query: String,
    limit: Option<i64>,
) -> AppResult<SearchResult> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::search(conn, &query, limit.unwrap_or(60)))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn search_suggestions(state: State<'_, AppState>, query: String) -> AppResult<Vec<SearchSuggestion>> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || db.with(|conn| repo::suggestions(conn, &query)))
        .await
        .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn saved_searches(state: State<'_, AppState>) -> AppResult<Vec<SavedSearch>> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || db.with(|conn| repo::list_saved_searches(conn)))
        .await
        .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn save_search(
    state: State<'_, AppState>,
    name: String,
    query: String,
) -> AppResult<SavedSearch> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::save_search(conn, &name, &query))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn delete_saved_search(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || db.with(|conn| repo::delete_saved_search(conn, &id)))
        .await
        .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn reading_orders(state: State<'_, AppState>) -> AppResult<Vec<ReadingOrder>> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || db.with(|conn| repo::orders(conn)))
        .await
        .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn reading_order(state: State<'_, AppState>, order_id: String) -> AppResult<ReadingOrderDetail> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || db.with(|conn| repo::order_detail(conn, &order_id)))
        .await
        .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn create_reading_order(
    state: State<'_, AppState>,
    name: String,
    description: Option<String>,
    kind: Option<String>,
) -> AppResult<ReadingOrder> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| {
            repo::create_order(
                conn,
                &name,
                description.as_deref(),
                kind.as_deref().unwrap_or("chronological"),
            )
        })
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn generate_publication_order(
    state: State<'_, AppState>,
    name: String,
) -> AppResult<ReadingOrderDetail> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::build_publication_order(conn, &name))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn delete_reading_order(state: State<'_, AppState>, order_id: String) -> AppResult<()> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || db.with(|conn| repo::delete_order(conn, &order_id)))
        .await
        .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn add_order_items(
    state: State<'_, AppState>,
    order_id: String,
    issue_ids: Vec<String>,
) -> AppResult<()> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::add_order_items(conn, &order_id, &issue_ids))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn reorder_items(
    state: State<'_, AppState>,
    order_id: String,
    issue_ids: Vec<String>,
) -> AppResult<()> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::reorder_order(conn, &order_id, &issue_ids))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn next_in_order(
    state: State<'_, AppState>,
    order_id: String,
    issue_id: String,
) -> AppResult<Option<NextIssue>> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::next_in_order(conn, &order_id, &issue_id))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn collections(state: State<'_, AppState>) -> AppResult<Vec<Collection>> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || db.with(|conn| repo::collections(conn)))
        .await
        .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn collection(state: State<'_, AppState>, collection_id: String) -> AppResult<CollectionDetail> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::collection_detail(conn, &collection_id))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn create_collection(
    state: State<'_, AppState>,
    name: String,
    description: Option<String>,
) -> AppResult<Collection> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::create_collection(conn, &name, description.as_deref()))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn update_collection(
    state: State<'_, AppState>,
    collection_id: String,
    name: Option<String>,
    description: Option<String>,
    cover_file_id: Option<String>,
) -> AppResult<Collection> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| {
            repo::update_collection(
                conn,
                &collection_id,
                name.as_deref(),
                description.as_deref(),
                cover_file_id.as_deref(),
            )
        })
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn delete_collection(state: State<'_, AppState>, collection_id: String) -> AppResult<()> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::delete_collection(conn, &collection_id))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn add_to_collection(
    state: State<'_, AppState>,
    collection_id: String,
    issue_ids: Vec<String>,
) -> AppResult<()> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::add_to_collection(conn, &collection_id, &issue_ids))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn add_series_to_collection(
    state: State<'_, AppState>,
    collection_id: String,
    series_id: String,
) -> AppResult<()> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::add_series_to_collection(conn, &collection_id, &series_id))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn remove_from_collection(
    state: State<'_, AppState>,
    collection_id: String,
    issue_ids: Vec<String>,
) -> AppResult<()> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::remove_from_collection(conn, &collection_id, &issue_ids))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn reorder_collection(
    state: State<'_, AppState>,
    collection_id: String,
    issue_ids: Vec<String>,
) -> AppResult<()> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::reorder_collection(conn, &collection_id, &issue_ids))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> AppResult<Settings> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || db.with(|conn| repo::settings(conn)))
        .await
        .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn save_settings(state: State<'_, AppState>, settings: Settings) -> AppResult<Settings> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::save_settings(conn, &settings))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn bookmarks(state: State<'_, AppState>, issue_id: String) -> AppResult<Vec<serde_json::Value>> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || db.with(|conn| repo::bookmarks(conn, &issue_id)))
        .await
        .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn add_bookmark(
    state: State<'_, AppState>,
    issue_id: String,
    page: i64,
    label: Option<String>,
    note: Option<String>,
) -> AppResult<serde_json::Value> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| {
            repo::add_bookmark(conn, &issue_id, page, label.as_deref(), note.as_deref())
        })
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn update_bookmark_note(
    state: State<'_, AppState>,
    id: String,
    note: Option<String>,
) -> AppResult<serde_json::Value> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::update_bookmark_note(conn, &id, note.as_deref()))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn delete_bookmark(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || db.with(|conn| repo::delete_bookmark(conn, &id)))
        .await
        .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn reveal_spoiler(state: State<'_, AppState>, issue_id: String) -> AppResult<()> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || db.with(|conn| repo::reveal_spoiler(conn, &issue_id)))
        .await
        .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn reading_history(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> AppResult<HistoryPayload> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| repo::reading_history(conn, limit.unwrap_or(100)))
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn stats(state: State<'_, AppState>) -> AppResult<Stats> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || db.with(|conn| repo::stats(conn)))
        .await
        .map_err(|err| AppError::message(err.to_string()))?
}

#[tauri::command]
pub async fn export_library_to(state: State<'_, AppState>, path: String) -> AppResult<()> {
    let db = state.db.clone();
    let payload = tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| {
            let filter = LibraryFilter {
                search: String::new(),
                universe: None,
                publisher: None,
                series_id: None,
                status: None,
                sort: "series".into(),
                descending: false,
            };
            let snapshot = repo::library_snapshot(conn, &filter)?;
            Ok(serde_json::json!({
                "version": 1,
                "exportedAt": crate::db::now(),
                "series": snapshot.series,
                "issues": snapshot.issues,
                "totals": snapshot.totals,
            }))
        })
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))??;
    let text = serde_json::to_string_pretty(&payload)
        .map_err(|err| AppError::message(err.to_string()))?;
    std::fs::write(&path, text)?;
    Ok(())
}

#[tauri::command]
pub async fn export_library(state: State<'_, AppState>) -> AppResult<serde_json::Value> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.with(|conn| {
            let filter = LibraryFilter {
                search: String::new(),
                universe: None,
                publisher: None,
                series_id: None,
                status: None,
                sort: "series".into(),
                descending: false,
            };
            let snapshot = repo::library_snapshot(conn, &filter)?;
            Ok(serde_json::json!({
                "version": 1,
                "exportedAt": crate::db::now(),
                "series": snapshot.series,
                "issues": snapshot.issues,
                "totals": snapshot.totals,
            }))
        })
    })
    .await
    .map_err(|err| AppError::message(err.to_string()))?
}
