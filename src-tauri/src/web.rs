//! Optional, user-initiated web lookup for character summaries.
//!
//! This is the *only* outbound network path in the app, and it exists for one
//! narrow case: the comics in a series do not carry enough text to describe a
//! character, so the user asks for a public summary instead. Nothing here runs
//! during import, scan, search or reading — the reader must stay fully usable
//! with the network disabled (`ARCHITECTURE.md` §14.3).
//!
//! Wikipedia's REST/API endpoints are the only allowlisted hosts. Responses are
//! bounded in size and time, and the text is stored as plain text so provider
//! HTML never reaches the webview.

use std::time::Duration;

use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::models::CharacterSummary;

const API_HOST: &str = "https://en.wikipedia.org";
const MAX_BODY_BYTES: u64 = 256 * 1024;
const TIMEOUT: Duration = Duration::from_secs(8);
const USER_AGENT: &str = "PanelComicReader/0.1 (local library app)";

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .user_agent(USER_AGENT)
        .build()
        .into()
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    query: Option<SearchQuery>,
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    #[serde(default)]
    search: Vec<SearchHit>,
}

#[derive(Debug, Deserialize)]
struct SearchHit {
    title: String,
}

#[derive(Debug, Deserialize)]
struct SummaryResponse {
    title: String,
    #[serde(default)]
    extract: String,
    content_urls: Option<ContentUrls>,
}

#[derive(Debug, Deserialize)]
struct ContentUrls {
    desktop: Option<DesktopUrls>,
}

#[derive(Debug, Deserialize)]
struct DesktopUrls {
    page: Option<String>,
}

fn get_text(url: &str) -> AppResult<String> {
    let mut response = agent()
        .get(url)
        .call()
        .map_err(|err| AppError::message(network_message(err)))?;
    let body = response
        .body_mut()
        .with_config()
        .limit(MAX_BODY_BYTES)
        .read_to_string()
        .map_err(|err| AppError::message(format!("could not read provider response: {err}")))?;
    Ok(body)
}

fn network_message(err: ureq::Error) -> String {
    match err {
        ureq::Error::StatusCode(code) => format!("provider returned HTTP {code}"),
        ureq::Error::Timeout(_) => "provider request timed out".to_string(),
        ureq::Error::HostNotFound => "provider host could not be reached".to_string(),
        other => format!("provider request failed: {other}"),
    }
}

/// Looks up a public summary for `query`.
///
/// Tries the page directly first (correct for "Sonic the Hedgehog"), then falls
/// back to a search hit so a slightly wrong name still resolves. Both requests
/// are bounded; any failure is returned as an error the UI shows verbatim
/// instead of inventing text.
pub fn lookup_character(query: &str) -> AppResult<CharacterSummary> {
    let name = query.trim();
    if name.is_empty() {
        return Err(AppError::invalid("character name is empty"));
    }
    if let Some(summary) = summary_for(name)? {
        return Ok(summary);
    }
    let search_url = format!(
        "{API_HOST}/w/api.php?action=query&list=search&format=json&srlimit=1&srsearch={}",
        encode_query(name)
    );
    let body = get_text(&search_url)?;
    let parsed: SearchResponse = serde_json::from_str(&body)
        .map_err(|err| AppError::message(format!("provider response was not JSON: {err}")))?;
    let title = parsed
        .query
        .and_then(|query| query.search.into_iter().next())
        .map(|hit| hit.title)
        .ok_or_else(|| AppError::not_found("no encyclopedia entry for that name"))?;
    summary_for(&title)?
        .ok_or_else(|| AppError::not_found("no encyclopedia entry for that name"))
}

fn summary_for(title: &str) -> AppResult<Option<CharacterSummary>> {
    let url = format!(
        "{API_HOST}/api/rest_v1/page/summary/{}",
        encode_path(title)
    );
    let body = match get_text(&url) {
        Ok(body) => body,
        // 404 is the normal "no such page" answer; anything else is a real error
        // the caller should see, so only swallow the not-found case.
        Err(AppError::Message(message)) if message.contains("HTTP 404") => return Ok(None),
        Err(err) => return Err(err),
    };
    let parsed: SummaryResponse = serde_json::from_str(&body)
        .map_err(|err| AppError::message(format!("provider response was not JSON: {err}")))?;
    let extract = parsed.extract.trim().to_string();
    if extract.is_empty() {
        return Ok(None);
    }
    let page = parsed
        .content_urls
        .and_then(|urls| urls.desktop)
        .and_then(|desktop| desktop.page)
        .unwrap_or_else(|| format!("{API_HOST}/wiki/{}", encode_path(&parsed.title)));
    Ok(Some(CharacterSummary {
        title: parsed.title,
        extract: truncate(&extract, 1200),
        url: page,
        source: "Wikipedia".into(),
    }))
}

/// Percent-encodes a path segment. Spaces become `%20`: a `+` in a REST path is
/// a literal plus, not a space.
fn encode_path(value: &str) -> String {
    encode(value, "%20")
}

/// Percent-encodes a query value, where `+` is the conventional space.
fn encode_query(value: &str) -> String {
    encode(value, "+")
}

fn encode(value: &str, space: &str) -> String {
    let mut out = String::new();
    for byte in value.trim().as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            b' ' => out.push_str(space),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut out: String = text.chars().take(limit).collect();
    if let Some(index) = out.rfind(' ') {
        out.truncate(index);
    }
    out.push('…');
    out
}
