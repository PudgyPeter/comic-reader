use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

pub const SCHEMA_VERSION: i64 = 13;

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
            if version < 10 {
                conn.execute_batch(MIGRATION_10)?;
            }
            if version < 11 {
                conn.execute_batch(MIGRATION_11)?;
            }
            if version < 12 {
                conn.execute_batch(MIGRATION_12)?;
            }
            if version < 13 {
                conn.execute_batch(MIGRATION_13)?;
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

// Multi-issue files (omnibus CBZ/CBR/PDF).
//
// The original schema tied one issue to one file with `file_id TEXT UNIQUE`.
// A single large archive can legitimately hold many chapters, so v10 rebuilds
// `issues` without that UNIQUE constraint and adds `page_offset` / `page_count`
// to carve the file into per-issue segments. NULL `page_count` means "to the end
// of the file", which is how every pre-v10 row stays valid.
//
// The table is rebuilt rather than ALTERed because SQLite cannot drop a UNIQUE
// constraint in place. Foreign keys are switched off for the rebuild (they
// cannot be changed inside a transaction) and the child tables keep referencing
// `issues` by name, so they bind to the replacement table after the rename.
const MIGRATION_10: &str = r#"
PRAGMA foreign_keys = OFF;

-- Triggers that reference `issues` but live on other tables outlive a DROP
-- TABLE issues, so they are removed up front and recreated afterwards.
DROP TRIGGER IF EXISTS series_fts_au;
DROP TRIGGER IF EXISTS bookmark_fts_ai;
DROP TRIGGER IF EXISTS bookmark_fts_ad;
DROP TRIGGER IF EXISTS bookmark_fts_au;
DROP TRIGGER IF EXISTS issue_fts_ai;
DROP TRIGGER IF EXISTS issue_fts_ad;
DROP TRIGGER IF EXISTS issue_fts_au;

CREATE TABLE issues_v10 (
    id TEXT PRIMARY KEY,
    file_id TEXT REFERENCES library_files(id) ON DELETE SET NULL,
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
    updated_at TEXT NOT NULL,
    variant_of TEXT REFERENCES issues(id) ON DELETE SET NULL,
    edition TEXT,
    metadata_locked INTEGER NOT NULL DEFAULT 0 CHECK (metadata_locked IN (0, 1)),
    page_offset INTEGER NOT NULL DEFAULT 0,
    page_count INTEGER
);

INSERT INTO issues_v10 (
    id, file_id, series_id, title, number, sort_number, volume, publisher, universe,
    release_date, writer, artist, cover_artist, description, created_at, updated_at,
    variant_of, edition, metadata_locked, page_offset, page_count
)
SELECT
    id, file_id, series_id, title, number, sort_number, volume, publisher, universe,
    release_date, writer, artist, cover_artist, description, created_at, updated_at,
    variant_of, edition, metadata_locked, 0, NULL
FROM issues;

DROP TABLE issues;
ALTER TABLE issues_v10 RENAME TO issues;

CREATE INDEX IF NOT EXISTS idx_issues_series ON issues(series_id, sort_number);
CREATE INDEX IF NOT EXISTS idx_issues_variant ON issues(variant_of);
CREATE INDEX IF NOT EXISTS idx_issues_file ON issues(file_id);

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

PRAGMA foreign_keys = ON;
"#;

// Characters and appearances (schema v11).
//
// The design documents place character profiles in LATER, but a minimal,
// *series-scoped* version is useful immediately: a ComicInfo.xml `<Characters>`
// tag already lists who appears in each issue, so the app can build a roster
// for a series without any provider, OCR or panel detection involved.
//
// Characters are scoped to one series (`series_id IS NULL` is reserved for a
// future cross-series universe database). Descriptions carry their own source
// and URL so a locally composed summary and a web-derived summary can coexist
// without pretending the web text came from the comics.
const MIGRATION_11: &str = r#"
CREATE TABLE IF NOT EXISTS characters (
    id TEXT PRIMARY KEY,
    series_id TEXT REFERENCES series(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    sort_name TEXT NOT NULL,
    description TEXT,
    description_source TEXT NOT NULL DEFAULT 'comic'
        CHECK (description_source IN ('comic', 'web', 'manual')),
    source_url TEXT,
    description_updated_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (series_id, name)
);

CREATE INDEX IF NOT EXISTS idx_characters_series ON characters(series_id, sort_name);

CREATE TABLE IF NOT EXISTS character_aliases (
    id TEXT PRIMARY KEY,
    character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    alias TEXT NOT NULL,
    alias_type TEXT NOT NULL DEFAULT 'alias'
        CHECK (alias_type IN ('alias', 'secret_identity', 'codename', 'title', 'misspelling')),
    UNIQUE (character_id, alias)
);

CREATE INDEX IF NOT EXISTS idx_character_aliases_alias ON character_aliases(alias);

CREATE TABLE IF NOT EXISTS character_appearances (
    id TEXT PRIMARY KEY,
    character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    issue_id TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    source TEXT NOT NULL DEFAULT 'comicinfo'
        CHECK (source IN ('comicinfo', 'manual')),
    note TEXT,
    created_at TEXT NOT NULL,
    UNIQUE (character_id, issue_id)
);

CREATE INDEX IF NOT EXISTS idx_appearances_issue ON character_appearances(issue_id);
CREATE INDEX IF NOT EXISTS idx_appearances_char ON character_appearances(character_id, issue_id);
"#;

// Character relationships (schema v12).
//
// `DATA_MODEL.md` §10.3 defines typed, directed edges with a reveal anchor.
// The shape here follows it with two pragmatic differences:
//
//   * IDs are TEXT UUIDs, matching the rest of this database, not INTEGERs.
//   * Direction is normalised in Rust (`characters::upsert_relationship`) rather
//     than by a CHECK: SQLite forbids subqueries in CHECK constraints, so a
//     "symmetric types store MIN as A" rule has to live in one repository
//     function. There is a trigger-free test for it.
//
// Relationship types are seeded so every installation starts with the same
// vocabulary; `code` is the stable identifier and the labels are what the UI
// shows. Symmetric types (`is_symmetric = 1`) render one edge regardless of
// which character the user opened.
const MIGRATION_12: &str = r#"
CREATE TABLE IF NOT EXISTS relationship_types (
    id            INTEGER PRIMARY KEY,
    code          TEXT NOT NULL UNIQUE,
    forward_label TEXT NOT NULL,
    reverse_label TEXT NOT NULL,
    is_symmetric  INTEGER NOT NULL DEFAULT 0 CHECK (is_symmetric IN (0,1))
);

INSERT OR IGNORE INTO relationship_types (id, code, forward_label, reverse_label, is_symmetric) VALUES
    (1, 'friend_of',      'Friend of',      'Friend of',     1),
    (2, 'ally_of',        'Ally of',        'Ally of',       1),
    (3, 'enemy_of',       'Enemy of',       'Enemy of',      1),
    (4, 'rival_of',       'Rival of',       'Rival of',      1),
    (5, 'family_of',      'Family of',      'Family of',     1),
    (6, 'member_of',      'Member of',      'Member of',     1),
    (7, 'romantic_with',  'Romantic with',  'Romantic with', 1),
    (8, 'mentor_of',      'Mentor of',      'Student of',    0),
    (9, 'parent_of',      'Parent of',      'Child of',      0),
    (10, 'sibling_of',    'Sibling of',     'Sibling of',    1),
    (11, 'teammate_of',   'Teammate of',    'Teammate of',   1);

CREATE TABLE IF NOT EXISTS character_relationships (
    id                   TEXT PRIMARY KEY,
    character_a_id       TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    character_b_id       TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    relationship_type_id INTEGER NOT NULL REFERENCES relationship_types(id),
    status               TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'former', 'unknown', 'secret')),
    description          TEXT,
    created_at           TEXT NOT NULL,
    updated_at           TEXT NOT NULL,
    CHECK (character_a_id <> character_b_id),
    UNIQUE (character_a_id, character_b_id, relationship_type_id)
);

CREATE INDEX IF NOT EXISTS idx_relationships_a ON character_relationships(character_a_id);
CREATE INDEX IF NOT EXISTS idx_relationships_b ON character_relationships(character_b_id);
"#;

// Provider support (schema v13).
//
// A provider lookup is *evidence*, not truth: it proposes character credits for
// an issue or a collected edition, and the user accepts or rejects them. Three
// things make that reviewable long after the fact:
//
//   * `provider_cache` keeps the raw response keyed by provider + resource, so a
//     rescan is free and an offline session never re-fetches. This is also what
//     makes ComicVine's 200-requests-per-resource-per-hour quota irrelevant.
//   * `provider_links` remembers which provider record a local row resolved to.
//     Resolving volume/issue identity is the expensive, error-prone step; doing
//     it once and remembering the answer is what stops a library from drifting
//     onto the wrong series.
//   * `provider_proposals` is the review queue. Nothing here writes to
//     `characters` or `character_appearances` until the user accepts.
//
// `characters.provider_id` / `provider` carry the global identity when a credit
// is accepted. Characters stay unique per `(series_id, name)` (v11), so the same
// character in two runs is still two rows; the provider columns are what make a
// future cross-series merge possible without guessing from names.
const MIGRATION_13: &str = r#"
ALTER TABLE characters ADD COLUMN provider TEXT;
ALTER TABLE characters ADD COLUMN provider_id TEXT;

CREATE INDEX IF NOT EXISTS idx_characters_provider
    ON characters(provider, provider_id);

CREATE TABLE IF NOT EXISTS provider_cache (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    resource TEXT NOT NULL,
    external_id TEXT NOT NULL,
    body TEXT NOT NULL,
    fetched_at TEXT NOT NULL,
    UNIQUE (provider, resource, external_id)
);

CREATE INDEX IF NOT EXISTS idx_provider_cache_fetched
    ON provider_cache(provider, fetched_at);

CREATE TABLE IF NOT EXISTS provider_links (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    -- 'series' or 'issue': which local table `local_id` points at.
    entity TEXT NOT NULL CHECK (entity IN ('series', 'issue')),
    local_id TEXT NOT NULL,
    external_id TEXT NOT NULL,
    -- How sure the resolver was, and the human-readable reason, so a wrong link
    -- is diagnosable without re-running the lookup.
    confidence REAL NOT NULL DEFAULT 0,
    reason TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (provider, entity, local_id)
);

CREATE INDEX IF NOT EXISTS idx_provider_links_external
    ON provider_links(provider, entity, external_id);

CREATE TABLE IF NOT EXISTS provider_proposals (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    -- 'chars' for a character-credit proposal set.
    kind TEXT NOT NULL,
    entity TEXT NOT NULL CHECK (entity IN ('series', 'issue')),
    local_id TEXT NOT NULL,
    -- JSON array of proposed names, so a set is accepted or rejected as a whole.
    payload TEXT NOT NULL,
    source_url TEXT,
    created_at TEXT NOT NULL,
    decided_at TEXT,
    decision TEXT CHECK (decision IN ('accepted', 'rejected'))
);

CREATE INDEX IF NOT EXISTS idx_provider_proposals_pending
    ON provider_proposals(entity, local_id, decision);
"#;

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
