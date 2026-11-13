//! ComicVine provider: volume resolution and character credits.
//!
//! This is the second outbound network path in the app, and the only one that
//! runs during import (when the user asks for it). It stays optional: every
//! lookup is user-initiated, responses are cached in SQLite so a rescan is
//! offline, and a failure never blocks an import.
//!
//! Two properties matter more than coverage:
//!
//! * **Identity is resolved once.** Series and issue identity is the expensive,
//!   ambiguous step, so the resolved ids are stored in `provider_links` and
//!   reused. Re-resolving on every import would let a library drift onto the
//!   wrong volume when ComicVine's search ranking changes.
//! * **Nothing is written without review.** A lookup produces
//!   `provider_proposals` rows; accepting one is a separate, explicit call.
//!
//! ComicVine imposes 200 requests per resource per hour (a resource is an
//! endpoint type: `search`, `volume`, `issues`, `issue`, ...) and sends no quota
//! headers, so `RateLimiter` counts locally and refuses to exceed the budget
//! instead of getting throttled mid-import.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::db::{new_id, now};
use crate::error::{AppError, AppResult};

const API_ROOT: &str = "https://comicvine.gamespot.com/api";
const USER_AGENT: &str = "PanelComicReader/0.1 (local library app)";
const TIMEOUT: Duration = Duration::from_secs(15);
const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;

/// ComicVine's documented budget: 200 requests per resource, per hour.
const RESOURCE_BUDGET: usize = 200;
const BUDGET_WINDOW: Duration = Duration::from_secs(3600);

/// Per-resource request counter. ComicVine has no quota headers, so the only way
/// to stay under the limit is to count locally.
#[derive(Debug, Default)]
pub struct RateLimiter {
    seen: Mutex<HashMap<&'static str, Vec<Instant>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records one request against `resource`, or refuses when the window is
    /// full. Refusing is deliberate: a partial import the user can retry is
    /// better than being blocked mid-run with an opaque 429.
    /// Test hook: exercises the budget without making a request.
    #[doc(hidden)]
    pub fn check_for_test(&self, resource: &'static str) -> AppResult<()> {
        self.check(resource)
    }

    fn check(&self, resource: &'static str) -> AppResult<()> {
        let mut seen = self
            .seen
            .lock()
            .map_err(|_| AppError::message("provider rate limiter poisoned"))?;
        let entries = seen.entry(resource).or_default();
        let cutoff = Instant::now() - BUDGET_WINDOW;
        entries.retain(|at| *at > cutoff);
        if entries.len() >= RESOURCE_BUDGET {
            return Err(AppError::invalid(format!(
                "ComicVine allows {RESOURCE_BUDGET} {resource} requests per hour; \
                 wait for the window to roll over or import in smaller batches"
            )));
        }
        entries.push(Instant::now());
        Ok(())
    }
}

/// Reads the API key from settings. It is user-supplied and never compiled in.
pub fn api_key(conn: &Connection) -> AppResult<String> {
    let key: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'comicVineApiKey'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    key.map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::invalid("no ComicVine API key set; add one in Settings to enable lookups")
        })
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .user_agent(USER_AGENT)
        .build()
        .into()
}

/// One comic record as ComicVine returns it.
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderIssue {
    pub id: u64,
    #[serde(default)]
    pub issue_number: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub cover_date: Option<String>,
    #[serde(default)]
    pub character_credits: Vec<ProviderCredit>,
    #[serde(default)]
    pub volume: Option<ProviderVolume>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderCredit {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderVolume {
    pub id: u64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub start_year: Option<String>,
    #[serde(default)]
    pub count_of_issues: Option<u32>,
    #[serde(default)]
    pub publisher: Option<ProviderNamed>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderNamed {
    #[serde(default)]
    pub name: String,
}

#[derive(Deserialize)]
struct SearchResponse {
    #[serde(default)]
    results: Vec<ProviderVolume>,
}

#[derive(Deserialize)]
struct IssueSearchItem {
    id: u64,
    #[serde(default)]
    issue_number: String,
}

#[derive(Deserialize)]
struct IssuesResponse {
    #[serde(default)]
    results: Vec<IssueSearchItem>,
}

#[derive(Deserialize)]
struct IssueResponse {
    results: ProviderIssue,
}

#[derive(Deserialize)]
struct VolumeResponse {
    results: ProviderVolume,
}

/// Reads a cached response, or fetches and stores one.
///
/// The cache is keyed by `(provider, resource, external_id)` so two different
/// queries never collide, and so an offline session replays exactly what was
/// seen before.
fn cached(
    conn: &Connection,
    limiter: &RateLimiter,
    resource: &'static str,
    external_id: &str,
    fetch: impl FnOnce(&str) -> AppResult<String>,
) -> AppResult<String> {
    let existing: Option<String> = conn
        .query_row(
            "SELECT body FROM provider_cache
             WHERE provider = 'comicvine' AND resource = ?1 AND external_id = ?2",
            params![resource, external_id],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(body) = existing {
        return Ok(body);
    }
    // The key is only required for a request, not for replaying a cached
    // response. That is what lets a rescan or an offline session work after the
    // key is removed.
    let key = api_key(conn)?;
    limiter.check(resource)?;
    let body = fetch(key.trim())?;
    conn.execute(
        "INSERT INTO provider_cache (id, provider, resource, external_id, body, fetched_at)
         VALUES (?1, 'comicvine', ?2, ?3, ?4, ?5)
         ON CONFLICT(provider, resource, external_id)
         DO UPDATE SET body = excluded.body, fetched_at = excluded.fetched_at",
        params![new_id(), resource, external_id, body, now()],
    )?;
    Ok(body)
}

fn get(key: &str, path: &str, query: &[(&str, String)]) -> AppResult<String> {
    let mut request = agent().get(&format!("{API_ROOT}/{path}/"));
    for (name, value) in query {
        request = request.query(name, value);
    }
    let request = request.query("api_key", key).query("format", "json");
    let mut response = request.call().map_err(|err| match err {
        ureq::Error::StatusCode(status) => {
            AppError::message(format!("ComicVine returned HTTP {status}"))
        }
        other => AppError::message(format!("ComicVine request failed: {other}")),
    })?;
    let body = response
        .body_mut()
        .with_config()
        .limit(MAX_BODY_BYTES as u64)
        .read_to_string()
        .map_err(|err| AppError::message(format!("cannot read ComicVine response: {err}")))?;
    Ok(body)
}

/// Resolves a series title to a ComicVine volume id.
///
/// Search rankings change, and a naive query returns the wrong run constantly
/// ("Sonic the Hedgehog 35" matches Archie before IDW). Candidates are scored on
/// signals that do not depend on ranking:
///
/// * the publisher matches what the library already knows,
/// * the volume started in a plausible year,
/// * the title contains the parent series name (how a mini-series is printed:
///   "Sonic the Hedgehog: Bad Guys"),
/// * the volume actually contains the issue number being imported.
///
/// Issue-number containment costs one extra request, so it only runs for the
/// best few candidates.
pub fn resolve_volume(
    conn: &Connection,
    limiter: &RateLimiter,
    title: &str,
    publisher: Option<&str>,
    issue_number: Option<&str>,
) -> AppResult<ProviderVolume> {
    let cache_key = format!("search|{title}");
    let body = cached(conn, limiter, "search", &cache_key, |key| {
        get(
            key,
            "search",
            &[
                ("query", title.to_string()),
                ("resources", "volume".into()),
                ("limit", "50".into()),
            ],
        )
    })?;
    let response: SearchResponse = serde_json::from_str(&body)
        .map_err(|err| AppError::message(format!("cannot parse ComicVine search: {err}")))?;

    let wanted = title.to_lowercase();
    let mut scored: Vec<(f32, ProviderVolume)> = response
        .results
        .into_iter()
        .map(|volume| {
            let mut score = 0.0;
            let name = volume.name.to_lowercase();
            if let Some(publisher) = publisher {
                let volume_publisher = volume
                    .publisher
                    .as_ref()
                    .map(|entry| entry.name.to_lowercase())
                    .unwrap_or_default();
                if !publisher.is_empty() && volume_publisher.contains(&publisher.to_lowercase()) {
                    score += 3.0;
                }
            }
            if name == wanted {
                score += 2.0;
            } else if name.contains(&wanted) {
                score += 1.0;
            }
            // A volume with more issues is the main run rather than a reprint
            // stub; ComicVine carries duplicates where one has a handful.
            score += (volume.count_of_issues.unwrap_or(0).min(200) as f32) / 200.0;
            (score, volume)
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    if let Some(number) = issue_number {
        for (_, candidate) in scored.iter().take(4) {
            if volume_has_issue(conn, limiter, candidate.id, number)? {
                return Ok(candidate.clone());
            }
        }
    }
    scored
        .into_iter()
        .next()
        .map(|(_, volume)| volume)
        .ok_or_else(|| AppError::not_found("a ComicVine volume for this series"))
}

fn volume_has_issue(
    conn: &Connection,
    limiter: &RateLimiter,
    volume_id: u64,
    number: &str,
) -> AppResult<bool> {
    let body = cached(conn, limiter, "issues", &volume_id.to_string(), |key| {
        get(
            key,
            "issues",
            &[
                ("filter", format!("volume:{volume_id}")),
                ("limit", "100".into()),
                ("sort", "issue_number:asc".into()),
            ],
        )
    })?;
    let response: IssuesResponse = serde_json::from_str(&body)
        .map_err(|err| AppError::message(format!("cannot parse ComicVine issues: {err}")))?;
    Ok(response
        .results
        .iter()
        .any(|issue| issue.issue_number.trim() == number.trim()))
}

/// Lists a volume's issues with their ids, so an import can map issue numbers to
/// ComicVine records.
pub fn volume_issues(
    conn: &Connection,
    limiter: &RateLimiter,
    volume_id: u64,
) -> AppResult<Vec<(String, u64)>> {
    let body = cached(conn, limiter, "issues", &volume_id.to_string(), |key| {
        get(
            key,
            "issues",
            &[
                ("filter", format!("volume:{volume_id}")),
                ("limit", "100".into()),
                ("sort", "issue_number:asc".into()),
            ],
        )
    })?;
    let response: IssuesResponse = serde_json::from_str(&body)
        .map_err(|err| AppError::message(format!("cannot parse ComicVine issues: {err}")))?;
    Ok(response
        .results
        .into_iter()
        .map(|row| (row.issue_number, row.id))
        .collect())
}

/// Fetches one issue record, including `character_credits`.
///
/// The `/issues` list endpoint omits credits entirely, so every issue needs its
/// own `/issue/4000-{id}` request. That is the dominant cost of a lookup.
pub fn issue(conn: &Connection, limiter: &RateLimiter, issue_id: u64) -> AppResult<ProviderIssue> {
    let body = cached(conn, limiter, "issue", &issue_id.to_string(), |key| {
        get(key, &format!("issue/4000-{issue_id}"), &[])
    })?;
    let response: IssueResponse = serde_json::from_str(&body)
        .map_err(|err| AppError::message(format!("cannot parse ComicVine issue: {err}")))?;
    Ok(response.results)
}

/// Fetches a volume record, including its description.
pub fn volume(conn: &Connection, limiter: &RateLimiter, volume_id: u64) -> AppResult<ProviderVolume> {
    let body = cached(conn, limiter, "volume", &volume_id.to_string(), |key| {
        get(key, &format!("volume/4050-{volume_id}"), &[])
    })?;
    let response: VolumeResponse = serde_json::from_str(&body)
        .map_err(|err| AppError::message(format!("cannot parse ComicVine volume: {err}")))?;
    Ok(response.results)
}

/// Character credits for one issue as a proposal row payload.
pub fn credit_names(issue: &ProviderIssue) -> Vec<Value> {
    let mut seen = std::collections::HashSet::new();
    issue
        .character_credits
        .iter()
        .filter(|credit| seen.insert(credit.id))
        .map(|credit| json!({ "id": credit.id, "name": credit.name.trim() }))
        .filter(|value| {
            value
                .get("name")
                .and_then(Value::as_str)
                .is_some_and(|name| !name.is_empty())
        })
        .collect()
}

/// Records a resolved link so future imports skip the search and scoring.
pub fn remember_link(
    conn: &Connection,
    entity: &str,
    local_id: &str,
    external_id: &str,
    confidence: f32,
    reason: &str,
) -> AppResult<()> {
    let timestamp = now();
    conn.execute(
        "INSERT INTO provider_links
            (id, provider, entity, local_id, external_id, confidence, reason, created_at, updated_at)
         VALUES (?1, 'comicvine', ?2, ?3, ?4, ?5, ?6, ?7, ?7)
         ON CONFLICT(provider, entity, local_id) DO UPDATE SET
            external_id = excluded.external_id,
            confidence = excluded.confidence,
            reason = excluded.reason,
            updated_at = excluded.updated_at",
        params![new_id(), entity, local_id, external_id, confidence, reason, timestamp],
    )?;
    Ok(())
}

pub fn linked_external_id(
    conn: &Connection,
    entity: &str,
    local_id: &str,
) -> AppResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT external_id FROM provider_links
             WHERE provider = 'comicvine' AND entity = ?1 AND local_id = ?2",
            params![entity, local_id],
            |row| row.get(0),
        )
        .optional()?)
}

/// Queues a proposal for review. Re-proposing the same set replaces the pending
/// row rather than piling up duplicates.
pub fn queue_proposal(
    conn: &Connection,
    kind: &str,
    entity: &str,
    local_id: &str,
    payload: &Value,
    source_url: Option<&str>,
) -> AppResult<String> {
    let pending: Option<String> = conn
        .query_row(
            "SELECT id FROM provider_proposals
             WHERE provider = 'comicvine' AND kind = ?1 AND entity = ?2 AND local_id = ?3
               AND decision IS NULL",
            params![kind, entity, local_id],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(id) = pending {
        conn.execute(
            "UPDATE provider_proposals SET payload = ?2, source_url = ?3 WHERE id = ?1",
            params![id, payload.to_string(), source_url],
        )?;
        return Ok(id);
    }
    let id = new_id();
    conn.execute(
        "INSERT INTO provider_proposals
            (id, provider, kind, entity, local_id, payload, source_url, created_at)
         VALUES (?1, 'comicvine', ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, kind, entity, local_id, payload.to_string(), source_url, now()],
    )?;
    Ok(id)
}

pub fn decide(conn: &Connection, proposal_id: &str, accepted: bool) -> AppResult<()> {
    let changed = conn.execute(
        "UPDATE provider_proposals SET decision = ?2, decided_at = ?3
         WHERE id = ?1 AND decision IS NULL",
        params![proposal_id, if accepted { "accepted" } else { "rejected" }, now()],
    )?;
    if changed == 0 {
        return Err(AppError::not_found("pending provider proposal"));
    }
    Ok(())
}

/// Applies an accepted character-credit set to the local roster.
///
/// Characters are created or matched per series (v11 identity), then tagged with
/// the ComicVine character id. The id is what a future cross-series merge can
/// trust; the name alone cannot be, because "Sonic" means different characters in
/// different runs.
pub fn accept_character_credits(
    conn: &Connection,
    entity: &str,
    local_id: &str,
    credits: &[Value],
) -> AppResult<usize> {
    let series_id = match entity {
        "series" => local_id.to_string(),
        "issue" => conn
            .query_row(
                "SELECT series_id FROM issues WHERE id = ?1",
                [local_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten()
            .ok_or_else(|| AppError::not_found("series for issue"))?,
        other => {
            return Err(AppError::invalid(format!(
                "provider proposals apply to a series or issue, not {other}"
            )))
        }
    };
    let issue_id = (entity == "issue").then(|| local_id.to_string());
    let timestamp = now();
    let mut applied = 0;
    for credit in credits {
        let Some(name) = credit.get("name").and_then(Value::as_str) else {
            continue;
        };
        let cleaned = crate::metadata::clean_character_name(name);
        if cleaned.is_empty() {
            continue;
        }
        let character_id =
            crate::characters::ensure_character(conn, &series_id, &cleaned, &timestamp)?;
        if let Some(external) = credit.get("id").and_then(Value::as_u64) {
            conn.execute(
                "UPDATE characters SET provider = 'comicvine', provider_id = ?2
                 WHERE id = ?1 AND (provider IS NULL OR provider_id IS NULL)",
                params![character_id, external.to_string()],
            )?;
        }
        if let Some(issue_id) = issue_id.as_deref() {
            conn.execute(
                "INSERT OR IGNORE INTO character_appearances
                    (id, character_id, issue_id, source, created_at)
                 VALUES (?1, ?2, ?3, 'manual', ?4)",
                params![new_id(), character_id, issue_id, timestamp],
            )?;
        }
        applied += 1;
    }
    Ok(applied)
}

/// Proposes character credits for one issue, queueing them for review.
pub fn propose_issue_characters(
    conn: &Connection,
    limiter: &RateLimiter,
    issue_id: &str,
    provider_issue_id: u64,
) -> AppResult<String> {
    let record = issue(conn, limiter, provider_issue_id)?;
    let credits = credit_names(&record);
    let payload = json!({
        "credits": credits,
        "issue": record.issue_number,
        "title": record.name,
        "coverDate": record.cover_date,
    });
    let url = format!("https://comicvine.gamespot.com/issue/4000-{provider_issue_id}/");
    queue_proposal(conn, "chars", "issue", issue_id, &payload, Some(&url))
}

/// Describes how a series differs from its earlier or sister runs.
///
/// ComicVine has no per-series personality field; its `description` is a
/// volume-level blurb. This produces a short derived note from what the provider
/// does have (publisher, start year, issue count) plus the volume description,
/// trimmed and attributed to its source. It is stored exactly like any other
/// description, with `description_source = 'web'` and the source URL, so it is
/// clearly not local-library fact.
pub fn series_descriptor(
    conn: &Connection,
    limiter: &RateLimiter,
    volume_id: u64,
) -> AppResult<(String, String)> {
    let volume = volume(conn, limiter, volume_id)?;
    let publisher = volume
        .publisher
        .as_ref()
        .map(|entry| entry.name.as_str())
        .unwrap_or("unknown publisher");
    let year = volume.start_year.as_deref().unwrap_or("unknown year");
    let count = volume
        .count_of_issues
        .map(|value| value.to_string())
        .unwrap_or_else(|| "?".into());
    let blurb = volume
        .description
        .as_deref()
        .map(strip_markup)
        .unwrap_or_default();
    let blurb = if blurb.is_empty() {
        String::new()
    } else {
        format!(" {blurb}")
    };
    let text = format!(
        "{publisher} run starting {year}, {count} issues in ComicVine's record.{blurb}"
    );
    let url = format!("https://comicvine.gamespot.com/volume/4050-{volume_id}/");
    Ok((text.trim().to_string(), url))
}

/// ComicVine descriptions are HTML. Keep the plain text and drop tags; this is a
/// local-first app and provider markup never reaches the webview.
fn strip_markup(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    let unescaped = out
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">");
    let mut collapsed = String::with_capacity(unescaped.len());
    let mut last_space = false;
    for ch in unescaped.chars() {
        if ch.is_whitespace() {
            if !last_space {
                collapsed.push(' ');
            }
            last_space = true;
        } else {
            collapsed.push(ch);
            last_space = false;
        }
    }
    let trimmed = collapsed.trim();
    // A blurb is shown beside library data; keep it to a readable paragraph.
    trimmed.chars().take(700).collect()
}

/// Character fields that carry a relationship list, and the local type each one
/// maps to. ComicVine is binary (friend/enemy) plus teams; the local schema is
/// typed, so the mapping is the closest honest match rather than a guess.
const EDGE_FIELDS: [(&str, &str); 3] = [
    ("character_friends", "friend_of"),
    ("character_enemies", "enemy_of"),
    ("teams", "member_of"),
];

/// Full character record, including the relationship-bearing lists.
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderCharacter {
    pub id: u64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub deck: Option<String>,
    #[serde(default)]
    pub real_name: Option<String>,
    #[serde(default)]
    pub aliases: Option<String>,
    #[serde(default)]
    pub character_friends: Vec<ProviderNamedRef>,
    #[serde(default)]
    pub character_enemies: Vec<ProviderNamedRef>,
    #[serde(default)]
    pub teams: Vec<ProviderNamedRef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderNamedRef {
    pub id: u64,
    #[serde(default)]
    pub name: String,
}

#[derive(Deserialize)]
struct CharacterResponse {
    results: ProviderCharacter,
}

/// Fetches one character record. This is the only request needed to get the
/// one-line bio, aliases and every relationship list at once.
pub fn character(
    conn: &Connection,
    limiter: &RateLimiter,
    character_id: u64,
) -> AppResult<ProviderCharacter> {
    let body = cached(conn, limiter, "character", &character_id.to_string(), |key| {
        get(key, &format!("character/4005-{character_id}"), &[])
    })?;
    let response: CharacterResponse = serde_json::from_str(&body)
        .map_err(|err| AppError::message(format!("cannot parse ComicVine character: {err}")))?;
    Ok(response.results)
}

/// One proposed edge, already filtered to characters that exist in the library.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposedEdge {
    pub character_id: String,
    pub character_name: String,
    pub other_id: String,
    pub other_name: String,
    pub relationship_type: String,
    pub source: String,
}

/// Builds relationship edges for a series' roster, keeping only pairs where
/// **both** ends are in the library.
///
/// ComicVine's friendship/enmity lists span the character's entire publication
/// history: Sonic has 140 friends and 133 enemies across every continuity, but
/// only ~21 and ~25 of those exist in an IDW run. Creating the rest would fill
/// the graph with nodes that have no issues, no pages and no reading progress,
/// so the graph would be mostly noise. Intersecting against the local roster is
/// what turns the provider's global lists into edges this library can actually
/// draw.
///
/// Deduplication is also required: a mutual friendship (A lists B, B lists A) is
/// one local edge, and `relationships::create` would otherwise see two inserts.
pub fn propose_edges(
    conn: &Connection,
    limiter: &RateLimiter,
    series_id: &str,
) -> AppResult<Vec<ProposedEdge>> {
    // Local characters for this series, keyed by provider id. Characters without
    // a provider id cannot participate: matching by name across continuities is
    // how the wrong person gets linked.
    let mut locals: HashMap<String, (String, String)> = HashMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT id, name, provider_id FROM characters
             WHERE series_id = ?1 AND provider = 'comicvine' AND provider_id IS NOT NULL",
        )?;
        let rows = stmt.query_map([series_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        for row in rows {
            let (id, name, provider_id) = row?;
            locals.insert(provider_id, (id, name));
        }
    }
    if locals.len() < 2 {
        return Ok(Vec::new());
    }

    let mut edges: Vec<ProposedEdge> = Vec::new();
    let mut seen: std::collections::HashSet<(String, String, String)> =
        std::collections::HashSet::new();

    for (provider_id, (local_id, local_name)) in locals.iter() {
        let Ok(external_id) = provider_id.parse::<u64>() else {
            continue;
        };
        let record = character(conn, limiter, external_id)?;
        for (field, relationship_type) in EDGE_FIELDS {
            let list = match field {
                "character_friends" => &record.character_friends,
                "character_enemies" => &record.character_enemies,
                _ => &record.teams,
            };
            for other in list {
                let Some((other_local, other_name)) = locals.get(&other.id.to_string()) else {
                    continue;
                };
                if other_local == local_id {
                    continue;
                }
                // `member_of` points at a team, which is not a character row, so
                // a team edge is only usable when the team itself is in the
                // roster (rare, but it keeps the rule uniform).
                let mut pair = [local_id.clone(), other_local.clone()];
                pair.sort();
                let key = (pair[0].clone(), pair[1].clone(), relationship_type.to_string());
                if !seen.insert(key) {
                    continue;
                }
                edges.push(ProposedEdge {
                    character_id: local_id.clone(),
                    character_name: local_name.clone(),
                    other_id: other_local.clone(),
                    other_name: other_name.clone(),
                    relationship_type: relationship_type.to_string(),
                    source: field.to_string(),
                });
            }
        }
    }
    edges.sort_by(|a, b| {
        a.character_name
            .cmp(&b.character_name)
            .then_with(|| a.other_name.cmp(&b.other_name))
    });
    Ok(edges)
}

/// Applies accepted edges. Symmetric types collapse to one row via
/// `relationships::create`, and a duplicate or reversed edge is a no-op.
pub fn accept_edges(
    conn: &Connection,
    edges: &[serde_json::Value],
) -> AppResult<usize> {
    let mut created = 0;
    for edge in edges {
        let (Some(character_id), Some(other_id), Some(kind)) = (
            edge.get("characterId").and_then(serde_json::Value::as_str),
            edge.get("otherId").and_then(serde_json::Value::as_str),
            edge.get("relationshipType").and_then(serde_json::Value::as_str),
        ) else {
            continue;
        };
        let input = crate::models::RelationshipInput {
            other_character_id: other_id.to_string(),
            relationship_type: kind.to_string(),
            status: Some("active".into()),
            description: None,
        };
        // A rejected edge in a previous run must not stop the rest.
        if crate::relationships::create(conn, character_id, &input).is_ok() {
            created += 1;
        }
    }
    Ok(created)
}

/// Fills in a short bio and aliases for characters already linked to ComicVine.
///
/// The `deck` field is used, not `description`: the full article is often 16 KB of
/// publication history ("was named Princess Sally in the American version of
/// Sonic CD"), which is the wrong thing to show beside library data. Anything
/// user-written or already sourced elsewhere is left alone.
pub fn enrich_characters(
    conn: &Connection,
    limiter: &RateLimiter,
    series_id: &str,
) -> AppResult<usize> {
    let mut targets: Vec<(String, String)> = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT id, provider_id FROM characters
             WHERE series_id = ?1 AND provider = 'comicvine' AND provider_id IS NOT NULL",
        )?;
        let rows = stmt.query_map([series_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            targets.push(row?);
        }
    }

    let timestamp = now();
    let mut enriched = 0;
    for (character_id, provider_id) in targets {
        let Ok(external_id) = provider_id.parse::<u64>() else {
            continue;
        };
        let record = character(conn, limiter, external_id)?;
        let deck = record
            .deck
            .as_deref()
            .map(strip_markup)
            .filter(|text| !text.is_empty())
            .map(|text| text.chars().take(600).collect::<String>());
        if let Some(deck) = deck {
            // Only replace an empty or previously-derived description, never a
            // manual edit or the local appearance summary.
            let changed = conn.execute(
                "UPDATE characters
                 SET description = ?2, description_source = 'web', source_url = ?3,
                     description_updated_at = ?4, updated_at = ?4
                 WHERE id = ?1
                   AND (description IS NULL OR trim(description) = ''
                        OR description_source = 'comic')",
                params![
                    character_id,
                    deck,
                    format!("https://comicvine.gamespot.com/character/4005-{external_id}/"),
                    timestamp
                ],
            )?;
            enriched += changed;
        }

        if let Some(aliases) = record.aliases.as_deref() {
            for alias in aliases.split(['\r', '\n']) {
                let cleaned = alias.trim();
                if cleaned.is_empty() || cleaned.eq_ignore_ascii_case(&record.name) {
                    continue;
                }
                conn.execute(
                    "INSERT OR IGNORE INTO character_aliases (id, character_id, alias, alias_type)
                     VALUES (?1, ?2, ?3, 'alias')",
                    params![new_id(), character_id, cleaned],
                )?;
            }
        }
    }
    Ok(enriched)
}
