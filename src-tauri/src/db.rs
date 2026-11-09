use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

pub const SCHEMA_VERSION: i64 = 9;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(dir: &Path) -> AppResult<Self> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join("library.sqlite3");
        if path.exists() {
            let backup_dir = dir.join("backups");
            let _ = std::fs::create_dir_all(&backup_dir);
            let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
            let _ = std::fs::copy(&path, backup_dir.join(format!("library-{stamp}.sqlite3")));
        }
        let conn = Connection::open(&path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "busy_timeout", 5000)?;
        let db = Database {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    pub fn with<T>(&self, f: impl FnOnce(&mut Connection) -> AppResult<T>) -> AppResult<T> {
        let mut guard = self
            .conn
            .lock()
            .map_err(|_| AppError::message("database lock poisoned"))?;
        f(&mut guard)
    }

    fn migrate(&self) -> AppResult<()> {
        self.with(|conn| {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
            )?;
            let version: i64 = conn
                .query_row(
                    "SELECT CAST(value AS INTEGER) FROM meta WHERE key = 'schema_version'",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            if version < 1 {
                conn.execute_batch(MIGRATION_1)?;
            }
            if version < 2 {
                conn.execute_batch(MIGRATION_2)?;
            }
            if version < 3 {
                conn.execute_batch(MIGRATION_3)?;
            }
            if version < 4 {
                conn.execute_batch(MIGRATION_4)?;
            }
            if version < 5 {
                conn.execute_batch(MIGRATION_5)?;
            }
            if version < 6 {
                conn.execute_batch(MIGRATION_6)?;
            }
            if version < 7 {
                conn.execute_batch(MIGRATION_7)?;
            }
            if version < 8 {
                conn.execute_batch(MIGRATION_8)?;
            }
            if version < 9 {
                conn.execute_batch(MIGRATION_9)?;
            }
            if version < SCHEMA_VERSION {
                conn.execute(
                    "INSERT INTO meta (key, value) VALUES ('schema_version', ?1)
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                    [SCHEMA_VERSION.to_string()],
                )?;
            }
            Ok(())
        })
    }
}

const MIGRATION_1: &str = r#"
CREATE TABLE IF NOT EXISTS library_roots (
    id TEXT PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    added_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS library_files (
    id TEXT PRIMARY KEY,
    root_id TEXT REFERENCES library_roots(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    relative_path TEXT NOT NULL,
    format TEXT NOT NULL,
    size_bytes INTEGER NOT NULL DEFAULT 0,
    page_count INTEGER NOT NULL DEFAULT 0,
    fingerprint TEXT,
    status TEXT NOT NULL DEFAULT 'present',
    added_at TEXT NOT NULL,
    scanned_at TEXT NOT NULL,
    UNIQUE (root_id, relative_path)
);
CREATE INDEX IF NOT EXISTS idx_files_status ON library_files(status);

CREATE TABLE IF NOT EXISTS file_pages (
    file_id TEXT NOT NULL REFERENCES library_files(id) ON DELETE CASCADE,
    page_index INTEGER NOT NULL,
    entry_name TEXT NOT NULL,
    width INTEGER NOT NULL DEFAULT 0,
    height INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (file_id, page_index)
);

CREATE TABLE IF NOT EXISTS series (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    sort_title TEXT NOT NULL,
    publisher TEXT,
    universe TEXT,
    description TEXT,
    volume TEXT,
    cover_file_id TEXT,
    cover_page INTEGER,
    year TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_series_unique
    ON series(lower(title), coalesce(publisher, ''), coalesce(universe, ''));

CREATE TABLE IF NOT EXISTS issues (
    id TEXT PRIMARY KEY,
    file_id TEXT UNIQUE REFERENCES library_files(id) ON DELETE SET NULL,
    series_id TEXT REFERENCES series(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    number TEXT NOT NULL DEFAULT '',
    sort_number REAL NOT NULL DEFAULT 0,
    volume TEXT,
    publisher TEXT,
    universe TEXT,
    release_date TEXT,
    writer TEXT,
    artist TEXT,
    cover_artist TEXT,
    description TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_issues_series ON issues(series_id, sort_number);

CREATE TABLE IF NOT EXISTS progress (
    issue_id TEXT PRIMARY KEY REFERENCES issues(id) ON DELETE CASCADE,
    page INTEGER NOT NULL DEFAULT 0,
    total INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'unread',
    started_at TEXT,
    completed_at TEXT,
    last_read_at TEXT,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS bookmarks (
    id TEXT PRIMARY KEY,
    issue_id TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    page INTEGER NOT NULL,
    label TEXT,
    note TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS reading_orders (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    kind TEXT NOT NULL DEFAULT 'chronological',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS reading_order_items (
    id TEXT PRIMARY KEY,
    order_id TEXT NOT NULL REFERENCES reading_orders(id) ON DELETE CASCADE,
    issue_id TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    position REAL NOT NULL,
    UNIQUE (order_id, issue_id)
);

CREATE TABLE IF NOT EXISTS collections (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    cover_file_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS collection_items (
    collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    issue_id TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    position REAL NOT NULL,
    PRIMARY KEY (collection_id, issue_id)
);

CREATE TABLE IF NOT EXISTS saved_searches (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    query TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS reading_sessions (
    id TEXT PRIMARY KEY,
    issue_id TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    started_at TEXT NOT NULL,
    ended_at TEXT NOT NULL,
    pages INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS spoiler_reveals (
    issue_id TEXT PRIMARY KEY REFERENCES issues(id) ON DELETE CASCADE,
    revealed_at TEXT NOT NULL
);
"#;

const MIGRATION_2: &str = r#"
CREATE TABLE IF NOT EXISTS file_preferences (
    file_id TEXT PRIMARY KEY REFERENCES library_files(id) ON DELETE CASCADE,
    spread_offset INTEGER NOT NULL DEFAULT 0,
    reading_direction TEXT,
    updated_at TEXT NOT NULL
);
"#;

const MIGRATION_4: &str = r#"
ALTER TABLE issues ADD COLUMN variant_of TEXT REFERENCES issues(id) ON DELETE SET NULL;
ALTER TABLE issues ADD COLUMN edition TEXT;
CREATE INDEX IF NOT EXISTS idx_issues_variant ON issues(variant_of);
"#;

const MIGRATION_3: &str = r#"
ALTER TABLE series ADD COLUMN reading_direction TEXT;
CREATE TABLE IF NOT EXISTS missing_issues (
    series_id TEXT NOT NULL REFERENCES series(id) ON DELETE CASCADE,
    number TEXT NOT NULL,
    reason TEXT NOT NULL DEFAULT 'missing',
    updated_at TEXT NOT NULL,
    PRIMARY KEY (series_id, number)
);
"#;

const MIGRATION_5: &str = r#"
ALTER TABLE progress ADD COLUMN scroll_offset REAL NOT NULL DEFAULT 0;
ALTER TABLE reading_sessions ADD COLUMN last_page INTEGER NOT NULL DEFAULT 0;
ALTER TABLE reading_sessions ADD COLUMN status TEXT NOT NULL DEFAULT 'closed';
ALTER TABLE reading_sessions ADD COLUMN resumed_at TEXT;
CREATE INDEX IF NOT EXISTS idx_sessions_status ON reading_sessions(status, ended_at);
"#;

const MIGRATION_6: &str = r#"
CREATE VIRTUAL TABLE IF NOT EXISTS issue_fts USING fts5(
    issue_id UNINDEXED,
    title,
    series,
    number,
    creator,
    publisher,
    description,
    notes,
    tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TRIGGER IF NOT EXISTS issue_fts_ai AFTER INSERT ON issues BEGIN
    INSERT INTO issue_fts (issue_id, title, series, number, creator, publisher, description, notes)
    VALUES (
        new.id,
        coalesce(new.title, ''),
        coalesce((SELECT title FROM series WHERE id = new.series_id), ''),
        coalesce(new.number, ''),
        trim(coalesce(new.writer, '') || ' ' || coalesce(new.artist, '') || ' ' || coalesce(new.cover_artist, '')),
        coalesce(new.publisher, ''),
        coalesce(new.description, ''),
        ''
    );
END;

CREATE TRIGGER IF NOT EXISTS issue_fts_ad AFTER DELETE ON issues BEGIN
    DELETE FROM issue_fts WHERE issue_id = old.id;
END;

CREATE TRIGGER IF NOT EXISTS issue_fts_au AFTER UPDATE ON issues BEGIN
    DELETE FROM issue_fts WHERE issue_id = old.id;
    INSERT INTO issue_fts (issue_id, title, series, number, creator, publisher, description, notes)
    VALUES (
        new.id,
        coalesce(new.title, ''),
        coalesce((SELECT title FROM series WHERE id = new.series_id), ''),
        coalesce(new.number, ''),
        trim(coalesce(new.writer, '') || ' ' || coalesce(new.artist, '') || ' ' || coalesce(new.cover_artist, '')),
        coalesce(new.publisher, ''),
        coalesce(new.description, ''),
        coalesce((SELECT group_concat(coalesce(label, '') || ' ' || coalesce(note, ''), ' ')
                  FROM bookmarks WHERE issue_id = new.id), '')
    );
END;

CREATE TRIGGER IF NOT EXISTS series_fts_au AFTER UPDATE OF title ON series BEGIN
    DELETE FROM issue_fts WHERE issue_id IN (SELECT id FROM issues WHERE series_id = new.id);
    INSERT INTO issue_fts (issue_id, title, series, number, creator, publisher, description, notes)
    SELECT i.id, coalesce(i.title, ''), coalesce(new.title, ''), coalesce(i.number, ''),
           trim(coalesce(i.writer, '') || ' ' || coalesce(i.artist, '') || ' ' || coalesce(i.cover_artist, '')),
           coalesce(i.publisher, ''), coalesce(i.description, ''),
           coalesce((SELECT group_concat(coalesce(label, '') || ' ' || coalesce(note, ''), ' ')
                     FROM bookmarks WHERE issue_id = i.id), '')
    FROM issues i WHERE i.series_id = new.id;
END;

CREATE TRIGGER IF NOT EXISTS bookmark_fts_ai AFTER INSERT ON bookmarks BEGIN
    UPDATE issue_fts SET notes = (
        SELECT group_concat(coalesce(label, '') || ' ' || coalesce(note, ''), ' ')
        FROM bookmarks WHERE issue_id = new.issue_id
    ) WHERE issue_id = new.issue_id;
END;

CREATE TRIGGER IF NOT EXISTS bookmark_fts_ad AFTER DELETE ON bookmarks BEGIN
    UPDATE issue_fts SET notes = coalesce((
        SELECT group_concat(coalesce(label, '') || ' ' || coalesce(note, ''), ' ')
        FROM bookmarks WHERE issue_id = old.issue_id
    ), '') WHERE issue_id = old.issue_id;
END;

CREATE TRIGGER IF NOT EXISTS bookmark_fts_au AFTER UPDATE OF label, note ON bookmarks BEGIN
    UPDATE issue_fts SET notes = (
        SELECT group_concat(coalesce(label, '') || ' ' || coalesce(note, ''), ' ')
        FROM bookmarks WHERE issue_id = new.issue_id
    ) WHERE issue_id = new.issue_id;
END;

-- Backfill existing rows so upgraded libraries are searchable immediately.
INSERT INTO issue_fts (issue_id, title, series, number, creator, publisher, description, notes)
SELECT i.id, coalesce(i.title, ''), coalesce(s.title, ''), coalesce(i.number, ''),
       trim(coalesce(i.writer, '') || ' ' || coalesce(i.artist, '') || ' ' || coalesce(i.cover_artist, '')),
       coalesce(i.publisher, ''), coalesce(i.description, ''),
       coalesce((SELECT group_concat(coalesce(b.label, '') || ' ' || coalesce(b.note, ''), ' ')
                 FROM bookmarks b WHERE b.issue_id = i.id), '')
FROM issues i LEFT JOIN series s ON s.id = i.series_id
WHERE NOT EXISTS (SELECT 1 FROM issue_fts f WHERE f.issue_id = i.id);

CREATE TABLE IF NOT EXISTS recent_searches (
    query TEXT PRIMARY KEY,
    searched_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_recent_searches_time ON recent_searches(searched_at DESC);
"#;

const MIGRATION_7: &str = r#"
ALTER TABLE issues ADD COLUMN metadata_locked INTEGER NOT NULL DEFAULT 0
    CHECK (metadata_locked IN (0, 1));
ALTER TABLE series ADD COLUMN metadata_locked INTEGER NOT NULL DEFAULT 0
    CHECK (metadata_locked IN (0, 1));
"#;

const MIGRATION_8: &str = r#"
ALTER TABLE collections ADD COLUMN is_smart INTEGER NOT NULL DEFAULT 0
    CHECK (is_smart IN (0, 1));
ALTER TABLE collections ADD COLUMN smart_rules TEXT;
ALTER TABLE collections ADD COLUMN sort_mode TEXT NOT NULL DEFAULT 'manual'
    CHECK (sort_mode IN ('manual', 'title', 'series', 'added_desc', 'added_asc', 'release_date'));
"#;

// Unmatched files are library_files rows with no issue pointing at them. Their
// progress and bookmarks live at file scope so they survive indexing, and a
// later "match to issue" moves these rows into the issue-scoped tables instead
// of recreating the file record (ARCHITECTURE.md §10.7).
const MIGRATION_9: &str = r#"
CREATE TABLE IF NOT EXISTS file_progress (
    file_id TEXT PRIMARY KEY REFERENCES library_files(id) ON DELETE CASCADE,
    page INTEGER NOT NULL DEFAULT 0,
    total INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'unread'
        CHECK (status IN ('unread', 'in-progress', 'completed')),
    started_at TEXT,
    completed_at TEXT,
    last_read_at TEXT,
    updated_at TEXT NOT NULL,
    scroll_offset REAL NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS file_bookmarks (
    id TEXT PRIMARY KEY,
    file_id TEXT NOT NULL REFERENCES library_files(id) ON DELETE CASCADE,
    page INTEGER NOT NULL,
    label TEXT,
    note TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_file_bookmarks_file ON file_bookmarks(file_id);
"#;

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
