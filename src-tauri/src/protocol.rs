use std::path::PathBuf;

use tauri::http::{header, Response, StatusCode};
use tauri::{Manager, UriSchemeContext, Wry};

use crate::commands::AppState;
use crate::repo;

fn error_response(status: StatusCode, message: &str) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(header::CACHE_CONTROL, "no-store")
        .body(message.as_bytes().to_vec())
        .unwrap_or_else(|_| Response::new(Vec::new()))
}

pub fn handle(ctx: UriSchemeContext<'_, Wry>, request: tauri::http::Request<Vec<u8>>) -> Response<Vec<u8>> {
    let path = request.uri().path().trim_start_matches('/');
    let segments: Vec<&str> = path.split('/').collect();
    let state = ctx.app_handle().state::<AppState>();
    let cache_dir: &PathBuf = &state.cache_dir;

    match segments.as_slice() {
        ["cover", file_id, page] | ["cover", file_id, page, _] => {
            let page = page.parse::<i64>().unwrap_or(0);
            let width = segments
                .get(3)
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(320);
            let cache_key = format!("{file_id}-{page}-{width}.jpg");
            let cache_path = cache_dir.join("covers").join(&cache_key);
            if let Ok(bytes) = std::fs::read(&cache_path) {
                return image_response(bytes, "image/jpeg", true);
            }
            match state.db.with(|conn| repo::cover_bytes(conn, file_id, page, width)) {
                Ok(bytes) => {
                    let _ = std::fs::create_dir_all(cache_dir.join("covers"));
                    let _ = std::fs::write(&cache_path, &bytes);
                    image_response(bytes, "image/jpeg", true)
                }
                Err(err) => error_response(StatusCode::NOT_FOUND, &err.to_string()),
            }
        }
        ["page", file_id, page] | ["page", file_id, page, _] => {
            let page = page.parse::<i64>().unwrap_or(0);
            let width = segments
                .get(3)
                .and_then(|value| value.parse::<u32>().ok());
            let Some(width) = width else {
                return match state.db.with(|conn| repo::page_bytes(conn, file_id, page)) {
                    Ok((bytes, mime)) => image_response(bytes, &mime, true),
                    Err(err) => error_response(StatusCode::NOT_FOUND, &err.to_string()),
                };
            };
            let cache_key = format!("{file_id}-{page}-w{width}.jpg");
            let cache_path = cache_dir.join("zoom").join(&cache_key);
            if let Ok(bytes) = std::fs::read(&cache_path) {
                return image_response(bytes, "image/jpeg", true);
            }
            match state
                .db
                .with(|conn| repo::page_bytes_scaled(conn, file_id, page, width))
            {
                Ok((bytes, mime)) => {
                    let _ = std::fs::create_dir_all(cache_dir.join("zoom"));
                    let _ = std::fs::write(&cache_path, &bytes);
                    image_response(bytes, &mime, true)
                }
                Err(err) => error_response(StatusCode::NOT_FOUND, &err.to_string()),
            }
        }
        _ => error_response(StatusCode::NOT_FOUND, "unknown resource"),
    }
}

fn image_response(bytes: Vec<u8>, mime: &str, cache: bool) -> Response<Vec<u8>> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(
            header::CACHE_CONTROL,
            if cache {
                "public, max-age=86400"
            } else {
                "no-store"
            },
        )
        .body(bytes)
        .unwrap_or_else(|_| Response::new(Vec::new()))
}
