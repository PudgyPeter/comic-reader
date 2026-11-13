use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRoot {
    pub id: String,
    pub path: String,
    pub added_at: String,
    pub file_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesSummary {
    pub id: String,
    pub title: String,
    pub publisher: Option<String>,
    pub universe: Option<String>,
    pub description: Option<String>,
    pub issue_count: i64,
    pub read_count: i64,
    pub cover_file_id: Option<String>,
    pub cover_page: Option<i64>,
    pub year: Option<String>,
    pub reading_direction: Option<String>,
    pub missing_count: i64,
    pub metadata_locked: bool,
}

/// A universe is a grouping label shared by series and issues; schema v11 and
/// earlier store it as free text, so a universe is derived, not its own table.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UniverseSummary {
    pub name: String,
    pub series_count: i64,
    pub issue_count: i64,
    pub read_count: i64,
    pub character_count: i64,
    pub publishers: Vec<String>,
    pub cover_file_id: Option<String>,
    pub cover_page: Option<i64>,
    pub year_first: Option<String>,
    pub year_last: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UniverseDetail {
    #[serde(flatten)]
    pub summary: UniverseSummary,
    pub series: Vec<SeriesSummary>,
    pub characters: Vec<CharacterCard>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesDetail {
    #[serde(flatten)]
    pub summary: SeriesSummary,
    pub issues: Vec<IssueCard>,
    pub missing: Vec<MissingIssue>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingIssue {
    pub number: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueCard {
    pub id: String,
    pub series_id: Option<String>,
    pub series_title: Option<String>,
    pub title: String,
    pub number: String,
    pub sort_number: f64,
    pub volume: Option<String>,
    pub publisher: Option<String>,
    pub universe: Option<String>,
    pub release_date: Option<String>,
    pub writer: Option<String>,
    pub artist: Option<String>,
    pub cover_artist: Option<String>,
    pub description: Option<String>,
    pub file_id: Option<String>,
    pub file_path: Option<String>,
    pub page_count: i64,
    pub last_page: i64,
    pub status: String,
    pub last_read_at: Option<String>,
    pub completed_at: Option<String>,
    pub spoiler_protected: bool,
    pub missing: bool,
    pub variant_of: Option<String>,
    pub edition: Option<String>,
    pub variant_count: i64,
    pub metadata_locked: bool,
    /// First page of this issue inside a shared archive file (multi-issue
    /// files). Zero for ordinary one-issue-per-file rows.
    pub page_offset: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportWarning {
    pub path: String,
    pub message: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub root: LibraryRoot,
    pub files_imported: i64,
    pub files_skipped: i64,
    pub series_created: i64,
    pub issues_created: i64,
    pub warnings: Vec<ImportWarning>,
}

/// Proposed metadata for one file that is not in the library yet. The import
/// review screen shows these, lets the user correct them, and sends the edited
/// values back with the commit request.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportCandidate {
    pub path: String,
    pub relative_path: String,
    pub format: String,
    pub page_count: i64,
    pub series_title: String,
    pub title: String,
    pub number: String,
    pub volume: Option<String>,
    pub publisher: Option<String>,
    pub release_date: Option<String>,
    pub writer: Option<String>,
    pub artist: Option<String>,
    pub cover_artist: Option<String>,
    pub description: Option<String>,
    pub duplicate: bool,
    pub warning: Option<String>,
}

/// User corrections keyed by `relative_path`. Any field left `None` keeps the
/// value proposed from ComicInfo.xml or the filename.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOverride {
    pub relative_path: String,
    pub series_title: Option<String>,
    pub title: Option<String>,
    pub number: Option<String>,
    pub volume: Option<String>,
    pub publisher: Option<String>,
    pub release_date: Option<String>,
    pub writer: Option<String>,
    pub artist: Option<String>,
    pub cover_artist: Option<String>,
    pub description: Option<String>,
    /// Import the file without creating an issue, so it lands in the unmatched
    /// view. The file is still readable and trackable.
    pub keep_unmatched: Option<bool>,
}

/// A library file that no issue references. It stays readable and keeps its own
/// progress/bookmarks until the user matches it to an issue, which moves the
/// existing file record instead of recreating it (ARCHITECTURE.md §10.7).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnmatchedFile {
    pub id: String,
    pub root_id: Option<String>,
    pub path: String,
    pub relative_path: String,
    pub format: String,
    pub size_bytes: i64,
    pub page_count: i64,
    pub status: String,
    pub added_at: String,
    pub page: i64,
    pub progress_status: String,
    pub last_read_at: Option<String>,
    pub suggested_series: String,
    pub suggested_number: String,
    pub suggested_title: String,
}

/// Metadata proposed for an unmatched file when the user opens the match flow.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileProposal {
    pub file_id: String,
    pub file_path: String,
    pub format: String,
    pub page_count: i64,
    pub series_title: String,
    pub title: String,
    pub number: String,
    pub volume: Option<String>,
    pub publisher: Option<String>,
    pub release_date: Option<String>,
    pub writer: Option<String>,
    pub artist: Option<String>,
    pub cover_artist: Option<String>,
    pub description: Option<String>,
}

/// Creates a new issue from an unmatched file. The file row is reused, so
/// progress and bookmarks carry over from `file_progress`/`file_bookmarks`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchFilePatch {
    pub series_title: Option<String>,
    pub series_id: Option<String>,
    pub title: Option<String>,
    pub number: Option<String>,
    pub volume: Option<String>,
    pub publisher: Option<String>,
    pub release_date: Option<String>,
    pub writer: Option<String>,
    pub artist: Option<String>,
    pub cover_artist: Option<String>,
    pub description: Option<String>,
}

/// One chapter row read from a contents page.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TocEntry {
    pub title: String,
    /// Series named by the row, when the contents page prints one. A collected
    /// edition can mix mini-series with the main run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,
    /// Issue number as printed ("35"), when the row names an issue.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub number: String,
    pub printed_page: i64,
}

/// A proposed issue split derived from a contents page, including the page
/// range it would own. Shown to the user before anything is written.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitProposal {
    pub file_id: String,
    pub toc_page: i64,
    pub page_count: i64,
    pub offset: i64,
    pub entries: Vec<SplitEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitEntry {
    pub title: String,
    /// Series named by the contents row. A collected edition can reprint
    /// several series, so this is per range rather than per file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,
    pub number: String,
    pub printed_page: i64,
    pub start_page: i64,
    pub end_page: i64,
}

/// A confirmed split: replaces the file's current issue with one issue per
/// range. Ranges are archive-absolute, half-open `[start, end)`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitRange {
    pub title: String,
    /// Series to file this range under. `None` falls back to the split's
    /// `series_title`, which is the file name.
    #[serde(default)]
    pub series: Option<String>,
    pub number: String,
    pub start_page: i64,
    pub end_page: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitRequest {
    pub file_id: String,
    pub series_title: Option<String>,
    pub ranges: Vec<SplitRange>,
}

/// A file-scoped progress state for unmatched files. Mirrors `ProgressState`
/// so the reader can treat both sources uniformly.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileProgressState {
    pub page: i64,
    pub total: i64,
    pub status: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub updated_at: Option<String>,
    #[serde(default)]
    pub scroll_offset: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySnapshot {
    pub issues: Vec<IssueCard>,
    pub series: Vec<SeriesSummary>,
    pub roots: Vec<LibraryRoot>,
    pub totals: LibraryTotals,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LibraryTotals {
    pub issues: i64,
    pub series: i64,
    pub completed: i64,
    pub in_progress: i64,
    pub unread: i64,
    pub pages_read: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub index: i64,
    pub name: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReaderPayload {
    pub file_id: String,
    pub issue_id: Option<String>,
    pub title: String,
    pub series_title: Option<String>,
    pub number: String,
    pub pages: Vec<PageInfo>,
    pub progress: ProgressState,
    pub background: String,
    pub mode: String,
    pub reading_direction: String,
    pub fit: String,
    pub transition: String,
    pub transition_duration: i64,
    pub show_interface: bool,
    pub next_issue_id: Option<String>,
    pub previous_issue_id: Option<String>,
    pub spread_offset: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressState {
    pub page: i64,
    pub total: i64,
    pub status: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub updated_at: Option<String>,
    #[serde(default)]
    pub scroll_offset: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NextIssue {
    pub id: String,
    pub title: String,
    pub number: String,
    pub series_title: Option<String>,
    pub file_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderItem {
    pub issue_id: String,
    pub position: f64,
    pub issue: Option<IssueCard>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingOrder {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub kind: String,
    pub item_count: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingOrderDetail {
    #[serde(flatten)]
    pub order: ReadingOrder,
    pub items: Vec<OrderItem>,
}

/// One typed rule in a smart collection. `field` and `op` are validated
/// against an allowlist before they reach SQL; values are always bound.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SmartRule {
    pub field: String,
    pub op: String,
    pub value: String,
}

/// Rules are combined with AND (`all`) and OR (`any`), then the two groups
/// are ANDed. An empty rule set matches nothing rather than everything, so a
/// half-built smart collection never dumps the whole library.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SmartRules {
    #[serde(default)]
    pub all: Vec<SmartRule>,
    #[serde(default)]
    pub any: Vec<SmartRule>,
}

impl SmartRules {
    pub fn is_empty(&self) -> bool {
        self.all.is_empty() && self.any.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CollectionPatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub cover_file_id: Option<String>,
    pub is_smart: Option<bool>,
    pub smart_rules: Option<SmartRules>,
    pub sort_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub cover_file_id: Option<String>,
    pub item_count: i64,
    pub created_at: String,
    pub is_smart: bool,
    pub smart_rules: Option<SmartRules>,
    pub sort_mode: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionDetail {
    #[serde(flatten)]
    pub collection: Collection,
    pub items: Vec<IssueCard>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub issues: Vec<IssueCard>,
    pub series: Vec<SeriesSummary>,
    pub total: i64,
    /// Matched excerpt per issue id, generated only after spoiler filtering.
    pub snippets: std::collections::BTreeMap<String, String>,
    /// Opaque cursor for the next page, absent when this is the last page.
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchFilter {
    #[serde(default)]
    pub query: String,
    pub publisher: Option<String>,
    pub series_id: Option<String>,
    pub universe: Option<String>,
    pub status: Option<String>,
    pub format: Option<String>,
    pub year_from: Option<i64>,
    pub year_to: Option<i64>,
    pub collection_id: Option<String>,
    #[serde(default)]
    pub sort: String,
    /// Opaque cursor from a previous `nextCursor`.
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentSearch {
    pub query: String,
    pub searched_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchSuggestion {
    pub value: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSearch {
    pub id: String,
    pub name: String,
    pub query: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub background: String,
    pub page_spacing: i64,
    pub brightness: f64,
    pub gamma: f64,
    pub reading_direction: String,
    pub default_mode: String,
    pub default_fit: String,
    pub default_zoom_behavior: String,
    pub transition: String,
    pub transition_duration: i64,
    pub animations_enabled: bool,
    pub show_interface: bool,
    pub fullscreen: bool,
    pub completion_threshold: f64,
    pub spoiler_protection: bool,
    pub spoiler_order_id: Option<String>,
    pub theme: String,
    pub accent: String,
    pub density: String,
    pub library_layout: String,
    pub library_sort: String,
    pub shortcuts: std::collections::BTreeMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        let mut shortcuts = std::collections::BTreeMap::new();
        shortcuts.insert("nextPage".into(), "ArrowRight, Space, PageDown".into());
        shortcuts.insert("prevPage".into(), "ArrowLeft, Shift+Space, PageUp".into());
        shortcuts.insert("nextIssue".into(), "Ctrl+ArrowRight".into());
        shortcuts.insert("prevIssue".into(), "Ctrl+ArrowLeft".into());
        shortcuts.insert("toggleFullscreen".into(), "F".into());
        shortcuts.insert("toggleZoom".into(), "Z".into());
        shortcuts.insert("escape".into(), "Escape".into());
        shortcuts.insert("bookmark".into(), "B".into());
        shortcuts.insert("search".into(), "Ctrl+F".into());
        Settings {
            background: "#0b0c10".into(),
            page_spacing: 16,
            brightness: 1.0,
            gamma: 1.0,
            reading_direction: "ltr".into(),
            default_mode: "single".into(),
            default_fit: "page".into(),
            default_zoom_behavior: "cursor".into(),
            transition: "slide".into(),
            transition_duration: 220,
            animations_enabled: true,
            show_interface: true,
            fullscreen: false,
            completion_threshold: 0.99,
            spoiler_protection: true,
            spoiler_order_id: None,
            theme: "dark".into(),
            accent: "#f59e0b".into(),
            density: "comfortable".into(),
            library_layout: "grid".into(),
            library_sort: "recent".into(),
            shortcuts,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub total_issues: i64,
    pub completed: i64,
    pub in_progress: i64,
    pub total_pages_read: i64,
    pub reading_seconds: i64,
    pub completed_this_month: i64,
    pub streak_days: i64,
    pub top_series: Vec<StatSeries>,
    pub by_status: Vec<StatBucket>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatSeries {
    pub name: String,
    pub read: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatBucket {
    pub label: String,
    pub count: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryFilter {
    #[serde(default)]
    pub search: String,
    pub universe: Option<String>,
    pub publisher: Option<String>,
    pub series_id: Option<String>,
    pub status: Option<String>,
    #[serde(default = "default_sort")]
    pub sort: String,
    #[serde(default)]
    pub descending: bool,
}

fn default_sort() -> String {
    "recent".into()
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuePatch {
    pub title: Option<String>,
    pub number: Option<String>,
    pub volume: Option<String>,
    pub publisher: Option<String>,
    pub release_date: Option<String>,
    pub writer: Option<String>,
    pub artist: Option<String>,
    pub cover_artist: Option<String>,
    pub description: Option<String>,
    pub series_id: Option<String>,
    pub series_title: Option<String>,
    pub variant_of: Option<String>,
    pub edition: Option<String>,
    pub metadata_locked: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesPatch {
    pub title: Option<String>,
    pub publisher: Option<String>,
    pub universe: Option<String>,
    pub description: Option<String>,
    pub volume: Option<String>,
    pub cover_file_id: Option<String>,
    pub reading_direction: Option<String>,
    pub metadata_locked: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistorySession {
    pub id: String,
    pub issue_id: String,
    pub title: String,
    pub series_title: Option<String>,
    pub number: String,
    pub file_id: Option<String>,
    pub started_at: String,
    pub ended_at: String,
    pub pages: i64,
    pub last_page: i64,
    pub status: String,
    pub resumed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPayload {
    pub sessions: Vec<HistorySession>,
    pub interrupted: Vec<HistorySession>,
    pub recent: Vec<IssueCard>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterCard {
    pub id: String,
    pub series_id: Option<String>,
    pub series_title: Option<String>,
    pub name: String,
    pub description: Option<String>,
    /// `comic` (derived from the library), `web` (fetched summary), `manual`.
    pub description_source: String,
    pub source_url: Option<String>,
    pub appearance_count: i64,
    pub read_count: i64,
    pub relationship_count: i64,
    pub description_updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterAppearance {
    pub issue_id: String,
    pub title: String,
    pub number: String,
    pub series_id: Option<String>,
    pub sort_number: f64,
    pub release_date: Option<String>,
    pub status: String,
    /// True when the issue is beyond the reading frontier; `title` is empty.
    pub spoiler_protected: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterDetail {
    #[serde(flatten)]
    pub card: CharacterCard,
    pub appearances: Vec<CharacterAppearance>,
    pub relationships: Vec<CharacterRelationship>,
}

/// One typed edge, resolved for the character whose page is open.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterRelationship {
    pub id: String,
    /// The other end of the edge.
    pub character_id: String,
    pub name: String,
    pub series_id: Option<String>,
    pub series_title: Option<String>,
    /// Stable type code, e.g. `parent_of`.
    pub relationship_type: String,
    /// `forward_label` or `reverse_label`, already resolved for this direction.
    pub label: String,
    pub status: String,
    pub description: Option<String>,
    pub is_symmetric: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipType {
    pub code: String,
    pub forward_label: String,
    pub reverse_label: String,
    pub is_symmetric: bool,
}

/// New edge from the open character to `other_character_id`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipInput {
    pub other_character_id: String,
    pub relationship_type: String,
    pub status: Option<String>,
    pub description: Option<String>,
}

/// Partial edit of an existing edge. `None` fields are left untouched.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipPatch {
    pub status: Option<String>,
    pub description: Option<String>,
}

/// User edit of a character. `None` fields are left untouched.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterPatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub description_source: Option<String>,
    pub source_url: Option<String>,
    pub aliases: Option<Vec<String>>,
}

/// A summary fetched from the web, before the user chooses to keep it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterSummary {
    pub title: String,
    pub extract: String,
    pub url: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatus {
    pub key_set: bool,
    pub pending_proposals: i64,
    pub cached_responses: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderLookup {
    pub external_id: String,
    pub name: String,
    pub publisher: Option<String>,
    pub start_year: Option<String>,
    pub issue_count: Option<i64>,
    /// Short series-level note derived from the provider record: publisher, run
    /// start and issue count, plus the trimmed volume blurb.
    pub descriptor: String,
    pub source_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderProposal {
    pub id: String,
    pub matched_issues: i64,
    pub local_issues: i64,
    pub credits: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderProposalRow {
    pub id: String,
    pub entity: String,
    pub local_id: String,
    pub credits: Vec<serde_json::Value>,
    pub source_url: Option<String>,
    pub created_at: String,
}
