pub mod archive;
pub mod characters;
pub mod commands;
pub mod db;
pub mod error;
pub mod library;
pub mod metadata;
pub mod models;
pub mod pdf;
pub mod protocol;
pub mod toc;
pub mod relationships;
pub mod repo;
pub mod web;

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
            // OCR models ship as bundled resources. The portable exe has no
            // installer, so also accept `models/` next to the executable, and in
            // dev fall back to the crate's own folder.
            let models_dir = [
                app.path()
                    .resolve("models", tauri::path::BaseDirectory::Resource)
                    .ok(),
                std::env::current_exe()
                    .ok()
                    .and_then(|exe| exe.parent().map(|dir| dir.join("models"))),
                Some(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("models")),
            ]
            .into_iter()
            .flatten()
            .find(|path| path.is_dir())
            .unwrap_or_else(|| data_dir.join("models"));
            let database = Database::open(&data_dir)?;
            app.manage(AppState {
                db: Arc::new(database),
                cache_dir,
                models_dir,
                import_control: Arc::new(library::ImportControl::new()),
            });
            Ok(())
        })
        .register_uri_scheme_protocol("comic", protocol::handle)
        .invoke_handler(tauri::generate_handler![
            commands::add_library_root,
            commands::preview_import,
            commands::commit_import,
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
            commands::refresh_issue_metadata,
            commands::delete_issue,
            commands::set_missing_reason,
            commands::reader_payload,
            commands::save_progress,
            commands::mark_issue_to_page,
            commands::start_reading_session,
            commands::touch_reading_session,
            commands::close_reading_session,
            commands::resume_reading_session,
            commands::issue_variants,
            commands::set_spread_offset,
            commands::set_direction_override,
            commands::set_issue_status,
            commands::set_series_direction,
            commands::set_series_status,
            commands::record_reading_session,
            commands::search_library,
            commands::search_filtered,
            commands::recent_searches,
            commands::clear_search_history,
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
            commands::create_smart_collection,
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
            commands::unmatched_files,
            commands::file_proposal,
            commands::match_file_to_new_issue,
            commands::match_file_to_existing_issue,
            commands::fileless_issues,
            commands::apply_file_match,
            commands::file_reader_payload,
            commands::save_file_progress,
            commands::file_bookmarks,
            commands::add_file_bookmark,
            commands::update_file_bookmark_note,
            commands::delete_file_bookmark,
            commands::set_file_spread_offset,
            commands::set_file_direction_override,
            commands::propose_split,
            commands::recombine_split,
            commands::apply_split,
            commands::split_available,
            commands::series_characters,
            commands::all_characters,
            commands::sync_series_characters,
            commands::character_detail,
            commands::update_character,
            commands::delete_character,
            commands::character_relationships,
            commands::relationship_types,
            commands::create_relationship,
            commands::update_relationship,
            commands::delete_relationship,
            commands::universes,
            commands::universe_detail,
            commands::lookup_character,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
