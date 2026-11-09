pub mod archive;
pub mod commands;
pub mod db;
pub mod error;
pub mod library;
pub mod metadata;
pub mod models;
pub mod pdf;
pub mod protocol;
pub mod repo;

use std::sync::Arc;

use tauri::Manager;

use crate::commands::AppState;
use crate::db::Database;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("app data directory is available");
            std::fs::create_dir_all(&data_dir)?;
            let cache_dir = data_dir.join("cache");
            std::fs::create_dir_all(&cache_dir)?;
            let database = Database::open(&data_dir)?;
            app.manage(AppState {
                db: Arc::new(database),
                cache_dir,
                import_control: Arc::new(library::ImportControl::new()),
            });
            Ok(())
        })
        .register_uri_scheme_protocol("comic", protocol::handle)
        .invoke_handler(tauri::generate_handler![
            commands::add_library_root,
            commands::cancel_import,
            commands::rescan_library,
            commands::remove_library_root,
            commands::library_roots,
            commands::library,
            commands::filter_options,
            commands::series_detail,
            commands::create_series,
            commands::update_series,
            commands::delete_series,
            commands::issue_detail,
            commands::update_issue,
            commands::bulk_update_issues,
            commands::delete_issue,
            commands::set_missing_reason,
            commands::reader_payload,
            commands::save_progress,
            commands::set_spread_offset,
            commands::set_direction_override,
            commands::set_issue_status,
            commands::set_series_direction,
            commands::set_series_status,
            commands::record_reading_session,
            commands::search_library,
            commands::search_suggestions,
            commands::saved_searches,
            commands::save_search,
            commands::delete_saved_search,
            commands::reading_orders,
            commands::reading_order,
            commands::create_reading_order,
            commands::generate_publication_order,
            commands::delete_reading_order,
            commands::add_order_items,
            commands::reorder_items,
            commands::next_in_order,
            commands::collections,
            commands::collection,
            commands::create_collection,
            commands::update_collection,
            commands::delete_collection,
            commands::add_to_collection,
            commands::add_series_to_collection,
            commands::remove_from_collection,
            commands::reorder_collection,
            commands::get_settings,
            commands::save_settings,
            commands::bookmarks,
            commands::add_bookmark,
            commands::delete_bookmark,
            commands::update_bookmark_note,
            commands::reveal_spoiler,
            commands::stats,
            commands::reading_history,
            commands::export_library,
            commands::export_library_to,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
