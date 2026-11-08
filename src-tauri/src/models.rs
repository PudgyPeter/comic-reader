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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub cover_file_id: Option<String>,
    pub item_count: i64,
    pub created_at: String,
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
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPayload {
    pub sessions: Vec<HistorySession>,
    pub recent: Vec<IssueCard>,
}
