# Comic Reader — Application Architecture

> **Status:** Proposed, for review. No application code has been written.
> **Basis:** Scaffold inspected at commit `ea22c53` ("Initial Tauri React TypeScript scaffold").
> **Companion documents:** [`DATA_MODEL.md`](./DATA_MODEL.md), [`ROADMAP.md`](./ROADMAP.md).

---

## 1. What the existing scaffold actually provides

The repository is an unmodified Tauri 2 template. There is no application architecture to
extend yet; we are choosing one.

| Area | Finding |
| --- | --- |
| Desktop shell | Tauri `2.12.5` (resolved in `src-tauri/Cargo.lock`), `tauri-plugin-opener 2.7.2` |
| Rust | Edition 2024, `rustc 1.99.1`, single `greet` command in `src-tauri/src/lib.rs` |
| Frontend | React `19.1`, TypeScript `~6.0.3`, Vite `8`, strict TS with `noUnusedLocals` / `noUnusedParameters` |
| Routing | None |
| State / data fetching | None beyond `useState` |
| Database | No SQLite, ORM, migration, or connection crates |
| Archives / images | No `zip`, `rar`, `image`, or image-processing crates |
| Networking | One outbound client: `ureq` in `web.rs` for the user-initiated character summary lookup (Wikipedia allowlist, 8 s timeout, 256 KiB cap). Nothing else uses the network. |
| File access | No dialog or filesystem plugin |
| Window | One window, `800x600`, no minimum size |
| Security | `csp: null`; capability grants only `core:default` and `opener:default` |
| Testing / linting | No test runner, no ESLint/Prettier, no CI configuration |
| Build | `npm run build` is `tsc && vite build`; dev server is fixed to port `1420` |
| Docs | `docs/` did not exist before this planning work |

**Implications:**

- We are free to choose the database, archive, image, routing, and state libraries. Nothing in
  the scaffold constrains those choices except Tauri 2, React 19, and TypeScript.
- The current security posture (`CSP: null`, broad default capability, no path validation) is
  acceptable for a hello-world template but not for an application that parses untrusted
  archives and displays remote metadata.
- A test harness and linting/formatting rules must be added before the first real feature.
- The demo `greet` command and template UI will eventually be removed, but this planning task
  does not modify them.

### 1.1 Foundational conclusion: files and catalog entries are different things

The most important architectural decision is not a library choice. It is this:

> A **library file** (a CBZ on disk) is a first-class object that can be read without any
> catalog metadata. An **issue** is an optional catalog identity that the file can be matched
> to.

A comic reader that only models "issues" breaks as soon as a user has an unrecognised file, a
fan translation, a scan without metadata, a convention ashcan, or a file containing three
issues. Conversely, a reader that only models files cannot support chronological reading
orders, story arcs, or character appearances.

The architecture therefore supports two entry points into the same content:

1. **File-first:** open a file and read it. No network, no metadata, no matching required.
2. **Catalog-first:** browse series/issues, reading orders, collections, and characters, with
   each catalog record linked to one or more files.

This separation is reflected in both the IPC contract and the database schema. It is a
**proposed decision requiring approval** (D1).

---

## 2. Goals and non-goals

### Goals

- **Local-first:** every core feature works with no network connection, including first launch
  after the library has been scanned.
- **File ownership:** the user's comic files stay where they are. The application indexes them;
  it does not take custody of them by default.
- **Spoiler-safe by construction:** spoiler filtering happens in Rust/SQL before data reaches
  React. It is not a CSS overlay.
- **Modular UI:** feature-oriented frontend modules, small components, explicit state machines.
- **Extensible catalog:** reading orders, characters, arcs, events, timelines, and locations can
  be added without reshaping the core tables.
- **Cross-platform desktop:** Windows, macOS, and Linux, with platform differences isolated
  behind Rust interfaces.

### Non-goals for the initial architecture

- No accounts, hosted backend, microservices, or mandatory cloud service.
- No always-on network dependency for reading, searching, or organising a library.
- No telemetry or analytics by default.
- No mobile target in the first phase, although the Tauri scaffold technically supports it.
- No multi-user server mode.

---

## 3. Architectural drivers

| Driver | Consequence |
| --- | --- |
| Offline reading is non-negotiable | All page bytes, thumbnails, progress, and catalog data are local. Metadata providers are optional enrichment. |
| Untrusted input | Archive parsing, image decoding, and metadata parsing happen in Rust with explicit resource limits. |
| Large libraries | Virtualised lists, paginated commands, indexed queries, background scans, and a thumbnail cache. |
| High-resolution page images | Image bytes must not cross the JSON IPC bridge. A custom protocol serves them. |
| Spoiler protection | Every query that can expose story information receives a viewer/profile context and filters in SQL. |
| Future features | Stable IDs, generic spoiler facts, an issue-level timeline model, and optional appearance/relationship tables are designed now. |
| Long-term maintainability | Thin IPC commands, domain services, repositories, and isolated platform adapters. |
| Local data durability | Migrations, backups, transactional writes, and corruption handling are part of the architecture, not an afterthought. |

---

## 4. System topology

```text
┌────────────────────────────────────────────────────────────────────────┐
│ Tauri desktop process (single OS process by default)                   │
│                                                                        │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ WebView (React 19 + TypeScript)                                  │  │
│  │                                                                  │  │
│  │  app shell / router                                              │  │
│  │  features: library, series, reader, search, collections, ...     │  │
│  │  query cache (server-state) + small UI stores                    │  │
│  │  reader layout strategies + keyboard command registry            │  │
│  └───────────────┬──────────────────────────────┬───────────────────┘  │
│                  │ Tauri IPC                    │ custom protocol      │
│                  │ invoke / events / channels   │ comic://             │
│  ┌───────────────▼──────────────────────────────▼───────────────────┐  │
│  │ Rust core                                                        │  │
│  │                                                                  │  │
│  │  ipc/            thin command layer, DTOs, error mapping         │  │
│  │  domain/         entities, rules, reading-order and spoiler logic│  │
│  │  db/             connection, migrations, repositories            │  │
│  │  library/        roots, scanner, watcher, matching               │  │
│  │  archive/        zip / rar / folder / pdf backends, page index   │  │
│  │  imaging/        decode, orient, thumbnails, page cache          │  │
│  │  metadata/       filename parser, ComicInfo, provider adapters   │  │
│  │  jobs/           background queue, cancellation, progress events │  │
│  │  security/       path validation, archive limits, allowlists     │  │
│  └───────┬───────────────────────┬───────────────────────┬───────────┘  │
│          │                       │                       │              │
│  ┌───────▼────────┐    ┌─────────▼──────────┐   ┌────────▼───────────┐  │
│  │ SQLite         │    │ Cache directory    │   │ User library files │  │
│  │ app data dir   │    │ thumbs, transformed│   │ read in place      │  │
│  │ library.db     │    │ pages, metadata    │   │ (CBZ, folders...)  │  │
│  └────────────────┘    └────────────────────┘   └────────────────────┘  │
│                                                                        │
│  Optional outbound network (metadata only, user-initiated or opt-in)   │
└────────────────────────────────────────────────────────────────────────┘
```

There is no separate server process. In development, Vite serves the UI on `localhost:1420`; in
production, the UI is bundled into the Tauri application and loaded by the WebView.

---

## 5. Frontend and backend responsibilities

| Concern | React / TypeScript | Rust |
| --- | --- | --- |
| Screen layout and navigation | Owns | — |
| Reader gestures, layout, zoom feel | Owns | Provides page dimensions and image URLs |
| Archive discovery and extraction | — | Owns |
| Image decoding, orientation, thumbnails | Displays results | Owns |
| SQLite access and migrations | — | Owns |
| Business rules (reading order, spoiler visibility, matching) | Presents results | Owns |
| Filesystem paths and permissions | Never sees arbitrary paths | Owns |
| Metadata HTTP requests | Triggers user actions | Owns requests, caching, rate limits |
| Keyboard shortcut definitions | Executes commands | — |
| Reading progress persistence | Decides when to save | Validates and persists |
| Error presentation | Owns | Produces typed errors |
| File watching | — | Owns |
| Platform-specific paths / dialogs | Uses returned handles | Owns |

**Rule:** React must not read comic files, parse archives, or construct filesystem paths.
Tauri's filesystem/asset plugins should not be used for comic content. The frontend addresses
content by opaque IDs (`fileId`, `pageIndex`) or by a custom protocol URL. This keeps archive
access and path validation in one place and avoids capability-scope sprawl.

---

## 6. Runtime and process model

- **Single process.** Tauri host + WebView, no Node runtime in production, no sidecar service.
- **Blocking work stays off the UI thread.** Archive reading, image decoding, SQLite calls, and
  filesystem scans run on `spawn_blocking` or a dedicated worker pool. Commands that do
  meaningful work are `async` and must not hold a database lock while decoding an image.
- **Background jobs** (scan, thumbnail generation, metadata matching) run in a Rust job queue
  with progress events, cancellation, and bounded concurrency.
- **One window initially.** The architecture keeps the reader as a route/feature rather than a
  window singleton so a detached reader window can be added later without moving business logic.
- **Development vs production.** Vite HMR only in development. Production uses the bundled
  frontend and a strict CSP. Any CSP that includes dev-server websockets must be scoped to dev.

---

## 7. Proposed module structure

### 7.1 Frontend

```text
src/
  app/                     application shell, providers, router, error boundaries
  features/
    library/               bookshelf, filters, import/scan UI
    series/                series list/detail, issue list
    reader/                reader shell, layout strategies, controls
      engine/              pagination, spread pairing, zoom, preload, transitions
      components/          page view, toolbar, thumbnails, settings panel
      hooks/               progress persistence, keyboard bindings, gestures
    search/                advanced search UI and facets
    collections/           custom collections
    reading-orders/        order editor, order progress, import/export
    characters/            later: profiles, relationship maps
    metadata/              match review, manual edit, provider settings
    settings/              global and profile preferences
  components/              shared presentational primitives only
  lib/
    ipc/                   generated DTO types, typed invoke wrapper, error mapping
    query/                 query keys, cache helpers
    keyboard/              command registry, scope resolution
  stores/                  small ephemeral UI stores (reader session, toasts)
  styles/                  tokens, themes, global styles
```

Rules:

- A feature owns its components, hooks, queries, and tests. Cross-feature imports go through
  `components/` or `lib/`, not through another feature's internals.
- **No component owns archive, database, or filesystem logic.**
- The reader is a composition of small components behind a state machine. The layout strategy
  (single page, spread, continuous strip) is a separate module with a common interface, so
  adding a mode does not add conditionals to the page renderer.
- Avoid a global god-store. Server data belongs to the query cache; only session/UI data belongs
  in a client store.

### 7.2 Rust

```text
src-tauri/src/
  main.rs                  process entry only
  lib.rs                   Tauri builder, plugins, state wiring
  ipc/
    mod.rs                 command registration
    dto.rs                 serde DTOs and conversion from domain types
    error.rs               AppError and IPC serialization
    commands/              one module per domain (library, reader, catalog, ...)
  domain/
    catalog/               publishers, series, issues
    library/               files, matching, progress rules
    reading/               orders, collections, spoiler visibility
    metadata/              provenance and conflict rules
  db/
    connection.rs           pool/actor, pragmas
    migrations/             embedded SQL migrations
    repositories/           SQL per aggregate; no SQL in commands
  library/
    scanner.rs              walk, stat, fingerprint, enqueue
    watcher.rs              later: filesystem notifications
    roots.rs                root validation and relink
  archive/
    mod.rs                  ArchiveBackend trait
    zip_backend.rs
    folder_backend.rs
    rar_backend.rs          optional, licensing-dependent
    limits.rs               page count / size / ratio limits
  imaging/
    decode.rs
    thumbnails.rs
    cache.rs
    protocol.rs             comic:// request handling
  metadata/
    comicinfo.rs
    filename.rs
    providers/              one adapter per source, behind a trait
    matcher.rs
  jobs/
    queue.rs
    progress.rs
  security/
    paths.rs
    validation.rs
```

Rules:

- IPC commands are thin: validate input, call a service/repository, map errors, return a DTO.
- Domain code never depends on Tauri types. Repositories never depend on the WebView.
- Archive backends implement one trait so CBZ, CBR, CB7, folders, and later PDF normalize to the
  same page-manifest interface.
- All platform-specific path handling is in Rust and tested on each target.

---

## 8. React ↔ Rust communication

### 8.1 Commands

Use Tauri commands for request/response operations. Proposed naming uses domain prefixes:

```text
library_list_roots        library_add_root         library_scan_root
library_list_files        library_get_file         library_relink_file
reader_open_file          reader_get_page_manifest reader_save_progress
catalog_get_series        catalog_list_issues      catalog_update_issue
reading_get_order         reading_reorder_item     reading_import_order
collections_list          collections_add_item
search_query              spoiler_get_facts
metadata_match_file       metadata_apply_match
```

Naming is illustrative, not final. The important parts:

- **DTOs use `#[serde(rename_all = "camelCase")]`** so TypeScript and Rust share one shape.
- **Pagination is explicit** (`cursor` or `offset` + `limit`, plus `total` only where needed).
  Never return an entire library in one command.
- **Paths do not cross the IPC boundary in either direction.** Folder selection returns a root
  ID; file operations take a file ID. Native dialogs are invoked by Rust or by the dialog plugin
  with the result immediately handed back to Rust.
- **Mutations return the updated DTO** so the query cache can be updated without a refetch.
- **Commands are idempotent where possible.** Progress writes, bookmark updates, and scan starts
  should tolerate retries.
- **Long-running commands return a job ID**, then report progress through events or a channel.

### 8.2 Events, channels, and binary data

| Need | Mechanism | Notes |
| --- | --- | --- |
| Scan/import progress | `Channel<JobEvent>` or job events | Channel per invocation is preferred; it avoids unrelated windows receiving the stream. |
| Library changed | `library:changed` event | Emitted after scan/import commits. Frontend invalidates relevant queries. |
| Progress changed elsewhere | `progress:changed` event | Used for multi-window and future sync; the originating window can update optimistically. |
| Page images | **Custom protocol** `comic://` | Never send image bytes through JSON. |
| Small thumbnails (rare fallback) | Custom protocol as well | Keep one image path. |

**Critical:** do not use global Tauri events as a general message bus for data. Events should
signal invalidation, not carry large payloads. The query cache then refetches authoritative data.

### 8.3 Custom protocol for pages and thumbnails

Proposed URLs:

```text
comic://page/<fileId>/<pageIndex>?maxWidth=<px>&format=<jpeg|webp|original>
comic://thumb/<fileId>?kind=cover|page&page=<n>&width=<px>
comic://asset/<assetId>            (later: character images, provider images)
```

The Rust handler:

1. Resolves the opaque ID through the database.
2. Confirms the file is under a registered root.
3. Opens/caches the archive and reads only the requested page.
4. Serves the original bytes when the WebView supports the format; otherwise serves a cached
   transcoded version.
5. Supports `ETag`, `Cache-Control: immutable`, and byte ranges.
6. Returns a typed 4xx/5xx response with no stack traces.

On Windows, Tauri custom protocols may be exposed as `http://comic.localhost/...`; the frontend
uses one helper to build URLs so platform differences are not scattered. The CSP must allow the
protocol on every platform.

**Why not `asset://`?** The asset protocol exposes files by path and requires scope management.
Comic pages live inside archives and must not be addressable as arbitrary files. A custom
protocol keeps the archive page index and the security boundary inside Rust. **Why not IPC
bytes?** Base64 or byte arrays in JSON add memory copies, block the IPC thread, and make range
requests/caching awkward. This is a proposed decision (D3).

### 8.4 Type safety across the boundary

The current scaffold has no shared type generation. Hand-written TypeScript interfaces will
drift. Two viable options:

- `specta` + `tauri-specta`: generates TypeScript bindings from Rust command signatures.
- `ts-rs`: derives TypeScript types from serde DTOs; command wrappers remain handwritten.

**Recommendation:** evaluate both in a spike before the first command is implemented; prefer the
one that generates command signatures and event payloads, not only structs. This is decision D7.
Until then, DTO definitions live in one Rust module and one TypeScript module with a contract
test that round-trips fixtures.

### 8.5 Contract rules

- Never expose database rows directly. DTOs are explicit and stable.
- Add a `schemaVersion` to expensive/complex payloads only if persisted or cached; do not version
  every IPC response.
- Breaking DTO changes require a frontend/backend update in the same release; this is a bundled
  desktop app, so no external API compatibility is promised.
- The frontend must handle unknown enum values defensively (display a fallback rather than
  crashing) because a newer database or imported data may contain values the UI predates.

---

## 9. Persistence strategy

### 9.1 Database choice

**Proposed decision:** SQLite in WAL mode, accessed through `rusqlite` with the `bundled`
feature. Embedding SQLite avoids a system dependency and gives deterministic behaviour across
Windows, macOS, and Linux.

Why not an ORM: the schema is relational and query-heavy; raw SQL with repositories is simpler
to reason about and to optimise. Why not `sqlx`: its async pool and compile-time query macros are
attractive, but the application already needs a blocking worker pool for archive/image work, and
`rusqlite` keeps the database layer synchronous and explicit. This is a trade-off, not a
certainty; see D2.

### 9.2 Connection and concurrency model

SQLite allows one writer at a time. The design must avoid the common Tauri failure mode where a
long scan transaction blocks every UI query.

- **One dedicated writer connection/worker** serialises all writes.
- **A small read pool (or a second read-only connection)** serves UI queries under WAL.
- **Transactions are short.** Never hold a transaction open while reading/decoding archive pages.
- **`PRAGMA foreign_keys = ON` on every connection.**
- **`PRAGMA journal_mode = WAL`, `synchronous = NORMAL`, `busy_timeout = 5000`.**
- **`PRAGMA temp_store = MEMORY`** and a bounded `mmap_size` are optional tuning knobs.
- **All DB calls from async commands use `spawn_blocking`.** Do not block the Tauri IPC runtime.

If a read pool proves unnecessary for the first release, a single connection behind a mutex is
acceptable as a starting point, but the repository interface must not assume a global lock.

### 9.3 Location, migrations, and durability

- Database: `app_data_dir()/library.db`.
- Cache: `app_cache_dir()/` (thumbnails, transcoded pages, provider snapshots if not in SQLite).
- Logs: `app_log_dir()/`.
- Migrations are forward-only, embedded in the binary, and run inside a transaction at startup.
  Use `refinery`/`rusqlite_migration` or a hand-rolled `PRAGMA user_version` runner; do not
  scatter ad-hoc `ALTER TABLE` calls.
- Before a migration, write a `VACUUM INTO` backup to a timestamped file. Keep a small number
  (for example, the last three) and never delete the only good backup.
- Run `PRAGMA quick_check` on startup only if it is cheap; schedule a full check in diagnostics.
- **Never place the live database in a cloud-synced folder** (Dropbox, iCloud Drive, OneDrive,
  network share). SQLite locking is unreliable there and corruption is a real risk. This should
  be detected/warned about where possible.

### 9.4 Identity and timestamps

- Internal primary keys are `INTEGER PRIMARY KEY` for compact indexes and fast joins.
- Every user-visible/sync-relevant row also has a stable `uuid TEXT UNIQUE`, generated by the
  application. UUIDs make export, relinking, and future multi-device sync possible without
  exposing integer IDs.
- Timestamps are `INTEGER` Unix milliseconds UTC. Calendar dates (cover dates, publication dates)
  are `TEXT` ISO-8601 (`YYYY-MM-DD`) because partial dates are common and SQLite has no date type.
- Soft delete uses `deleted_at` for user-meaningful records. Hard deletes are used for pure
  derived cache rows (thumbnails, page manifests on rescan) where appropriate.

### 9.5 Search

Use SQLite FTS5 (bundled with SQLite). The search index is a derived table, rebuilt on demand
and updated in the same transaction as catalog writes. Do not query `LIKE '%term%'` across the
catalog. Search results must be filtered through the spoiler policy **before** snippets are
generated; see `DATA_MODEL.md`.

### 9.6 What is not in the database

- Page image bytes.
- Thumbnails and transcoded page files.
- Raw provider responses that are large and only needed for re-parsing may live in a snapshot
  table if they are modest; otherwise use a cache file with a content hash. The default is a
  `metadata_snapshots` table with gzip-compressed JSON if size permits.
- Transient job state. Jobs are runtime state, not user data, unless a scan history is desired.

---

## 10. File and archive handling

### 10.1 Library roots and paths

- A **library root** is a user-selected folder, stored as a canonical absolute path with a
  display name.
- Files are stored as **root ID + relative path**, never as a standalone absolute path. Moving a
  root is then a single update; moving a file inside a root is detected during rescan.
- The scanner never writes to, renames, or deletes comic files. It is read-only with respect to
  the library.
- Symlinks: follow only if explicitly enabled, and detect cycles. Do not traverse outside a
  registered root through a symlink unless a root itself resolves there.

### 10.2 Scan pipeline

```text
walk root
  -> filter supported extensions / folders
  -> stat (size, mtime)
  -> cheap fingerprint (size + mtime + first/last chunk hash)
  -> upsert library_files
  -> read archive central directory / folder listing
  -> build page manifest (entry names, natural order, dimensions)
  -> parse ComicInfo.xml
  -> match to catalog issue (only if enabled)
  -> queue thumbnail generation
  -> emit progress and commit in batches
```

The scanner must be resumable after cancellation and must not leave a half-imported file visible
as complete. Use a per-file state (`pending`, `indexed`, `unreadable`) and commit page manifests
transactionally per file.

### 10.3 Identity, moves, and relinking

Content hashing is the reliable way to recognise a moved file. Strategy:

1. Cheap key: size + mtime + a hash of the first and last 64 KiB. Fast, but not proof.
2. Full content hash (BLAKE3 or xxHash3) computed in the background for matching and duplicate
   detection.
3. On rescan, a missing path plus a new path with the same full hash is treated as a move.
   If the hash is unavailable, present a manual relink flow.

File system inode/file IDs are not portable and should not be the primary identity.

### 10.4 Archive support

| Format | Phase | Approach | Risk |
| --- | --- | --- | --- |
| Folder of images | CORE | Directory listing, natural sort | Low |
| CBZ / ZIP | CORE | `zip` crate, random access, ZIP64 | Low |
| CB7 / 7z | CORE stretch / LATER | `sevenz-rust` or libarchive | Medium; solid archives may require full extraction |
| CBR / RAR | LATER / conditional | `unrar` binding or libarchive | **Licensing and redistribution risk; must be reviewed before shipping** |
| PDF | LATER | Separate renderer (PDFium/MuPDF) and a separate page cache | High: rendering model differs from image archives |
| Ugoira/animations | OPTIONAL | Defer | Low priority |

CBR support is not a simple "add a crate" decision. The UnRAR licence is not OSI-style and has
redistribution obligations. If RAR support is required at launch, get explicit approval and a
legal review; otherwise ship CBZ/folder first and show a clear unsupported-format message. This
is decision D5.

### 10.5 Archive hardening

Archives are untrusted input. Enforce limits before allocating memory or writing files:

- Maximum entries per archive and maximum page count.
- Maximum uncompressed size per page and per archive.
- Compression-ratio limit (zip-bomb defence).
- Reject or skip encrypted entries with a clear error.
- Never extract by entry name to a filesystem path. If extraction is ever needed, sanitise the
  entry name and verify the canonical target remains under the cache directory (zip-slip).
- Ignore `__MACOSX`, `.DS_Store`, hidden files, and `Thumbs.db`.
- Treat malformed XML in `ComicInfo.xml` as untrusted; disable external entities and cap size.
- Open archives with a bounded cache; close least-recently-used handles.

### 10.6 Page ordering

- Natural sort (`1.jpg`, `2.jpg`, `10.jpg`) is mandatory. Use a well-tested natural-order
  implementation, not lexicographic sorting.
- Support explicit ordering from `ComicInfo.xml` where present.
- Store the resolved page manifest (`file_pages`) so the order is stable even if the archive is
  later renamed internally.
- Double-page detection is a heuristic: very wide images, filename hints, or `ComicInfo` flags.
  Store the result as an overrideable flag, never as a permanent silent assumption.

### 10.7 Unmatched and loose files

Files that do not match an issue must still be browsable, readable, and trackable. The library
UI needs an "unmatched" view. Mapping a file to an issue later must preserve its progress and
bookmarks by moving the mapping, not recreating the file record.

### 10.8 File watching

Defer to LATER unless the scan is fast and manual rescan is acceptable for the first release.
When added, use `notify`, debounce events, and reconcile against a scan rather than trusting
individual events. Watchers are unreliable on network shares and some Linux filesystems.

---

## 11. Image rendering strategy

### 11.1 Delivery path

```text
React <img src="comic://...">
  -> Rust custom protocol handler
  -> resolve file/page from SQLite
  -> open archive (LRU cache)
  -> read entry bytes
  -> original, or decode/transcode to cache
  -> HTTP-like response with ETag + range support
```

Page bytes should be streamed, not loaded into the WebView via base64. The WebView is allowed to
decode images itself for common formats; Rust normalises unsupported formats to JPEG/WebP.

### 11.2 Format support is not uniform

Tauri uses the platform WebView:

- Windows: WebView2 (Chromium-based) — broad support.
- macOS: WKWebView — broad but not identical.
- Linux: WebKitGTK — the weakest support for newer formats and GPU-heavy effects; AVIF/JXL and
  some colour profiles may fail or be slow.

Therefore:

- JPEG, PNG, WebP, GIF are the safe baseline.
- AVIF, JXL, and unusual TIFF/CMYK JPEGs are transcoded in Rust to a cached WebP/JPEG.
- Orientation metadata is applied in Rust so the frontend never has to guess.
- Do not rely on CSS `image-rendering` or filters for correctness; they are cosmetic.

### 11.3 Cache design

- Thumbnails are content-addressed by file content hash + page index + recipe (size, format,
  quality). A changed recipe produces a new key; old entries are evicted by an LRU/size budget.
- Page transcodes use a similar key. Original JPEG/PNG pages are not duplicated.
- Cache writes are atomic: write to a temporary file, then rename.
- The cache is disposable. If it is deleted or corrupted, it regenerates.
- Thumbnail sizes: a small grid size (for example 320 px wide) and a larger detail size (640 px).
  Do not generate full-resolution thumbnails.

### 11.4 Page dimensions and spreads

The reader needs width/height for every page to:

- pair pages into spreads correctly,
- choose fit modes,
- avoid layout jumps,
- decide whether a page is a double-page spread.

Read dimensions cheaply during scan (header probing, similar to the `imagesize` approach) or
lazily on first open, and persist them in `file_pages`. Do not fully decode every page just to
read its size.

### 11.5 Zoom and high-resolution scans

The first implementation should use CSS transforms with a bounded decoded page. It is fast and
good enough for typical 1500–3000 px pages.

**Critical caveat:** CSS zoom does not create detail. A 6000 px scan downscaled by the WebView
for fit-to-screen can look soft when zoomed; re-decoding at a higher resolution or using tiles is
the real solution. Deep-zoom tiling (a page pyramid, DZI-like) is a significant subsystem:

- tile generation and cache,
- tile URL scheme and range requests,
- seamless compositing,
- memory budgets,
- keyboard/gesture interaction at tile boundaries.

**Recommendation:** ship transform-based zoom in CORE and evaluate tiling only after profiling
real large scans. Do not promise "infinite zoom" in the first release. This is decision D14.

### 11.6 Memory and preloading

- Keep a small window of decoded pages (current, next, previous); release the rest.
- Preload the next page during idle time, but cancel if the user navigates away.
- Bound the archive handle cache by both count and estimated memory.
- Continuous/webtoon mode needs stronger virtualisation; do not render hundreds of `<img>`
  elements at once.
- Monitor memory on Linux/WebKitGTK, where large images are the most likely source of instability.

### 11.7 Transitions and colour

- Transitions are CSS/DOM effects, not image processing.
- Respect `prefers-reduced-motion`.
- Avoid transitions during rapid page turns; they add perceived latency.
- Brightness/gamma controls should be non-destructive display filters. Do not rewrite page files.
- Full colour management (ICC) is a LATER evaluation; most comic reading does not need it.

---

## 12. Application state management

### 12.1 State layers

| Layer | Owner | Examples |
| --- | --- | --- |
| Server/catalog state | Query cache (proposed TanStack Query) | series list, issue detail, search results, reading orders |
| Ephemeral reader state | Reader state machine / small client store | current page, zoom, pan, mode, transition state |
| Persistent preferences | Rust/SQLite | profile defaults, per-series overrides, library roots |
| Navigation | Router | current route, selected series, reader target |
| Notifications | Small UI store | toasts, job progress, error banners |

The query cache is the source of truth for anything that comes from Rust. Do not copy the
catalog into a global store; that creates two sources of truth and stale UI.

### 12.2 Reader state machine

The reader should be an explicit state machine rather than a pile of booleans:

```text
idle -> loading -> ready -> (turning | zooming | panning) -> ready
                  \-> error
ready -> closed -> flush progress
```

Events: open file, page turn, mode change, zoom change, window resize, save progress, close.
This makes keyboard, gestures, and transitions predictable and testable.

### 12.3 Progress persistence

- Save on meaningful page changes immediately; throttle continuous scroll writes.
- Debounce scroll offset/zoom updates (for example 500–1000 ms) and flush on close, route change,
  window blur, and application suspend.
- The Rust command validates page bounds and maps physical pages to issues.
- Use an idempotent write keyed by `(profile_id, file_id)` so retries are safe.
- Never write progress on every scroll event; SQLite is fast but the IPC and disk churn are not
  free.

### 12.4 Routing and restoration

A desktop app still benefits from a router (`/library`, `/series/:id`, `/reader/:fileId`,
`/orders/:id`, `/settings`). It provides back/forward behaviour and deep links between features.
Restore the last route on startup; restore the last reading position only when the user chooses
to resume, not by silently opening a comic at launch.

Router choice (React Router vs TanStack Router) is not architecturally significant; pick one
before feature work and keep route definitions in `src/app/`. Prefer a typed router if the team
is comfortable with it.

### 12.5 Keyboard shortcuts

- A central command registry maps `commandId -> handler`, with scopes (`global`, `library`,
  `reader`, `dialog`) and user-remappable bindings.
- Detect conflicting bindings and show them in settings.
- Never hard-code key handlers inside random components.
- Provide a searchable shortcut cheat sheet.
- Ensure every mouse action has a keyboard equivalent and that focus is restored after dialogs.

### 12.6 Component discipline

- One component, one responsibility; containers fetch, presentational components render.
- Reader page rendering, layout calculation, zoom/pan, toolbar, and settings are separate.
- Avoid prop drilling beyond two levels; use feature context or query hooks.
- Use virtualisation for covers, issue lists, search results, and page strips.
- Keep effects out of render paths; progress writes belong in event handlers/state-machine
  transitions, not in a `useEffect` that fires on every render.

---

## 13. Error handling

### 13.1 Rust error model

Define one serialisable error type at the IPC boundary:

```text
AppError {
  code: string,          // stable machine-readable code
  message: string,       // safe, user-facing
  details?: object,      // structured context, no secrets
  retryable: boolean
}
```

Domain errors use `thiserror`; the IPC layer maps them. Proposed codes:

`NOT_FOUND`, `PERMISSION_DENIED`, `INVALID_INPUT`, `CONFLICT`, `UNSUPPORTED_FORMAT`,
`CORRUPT_ARCHIVE`, `FILE_MISSING`, `DB_BUSY`, `DB_ERROR`, `IO_ERROR`, `CANCELLED`,
`NETWORK_ERROR`, `PROVIDER_ERROR`, `INTERNAL`.

Rules:

- No `unwrap()`/`expect()` on user data, file contents, or database results. Panics are allowed
  only for true invariants, and `panic = "abort"` in release means a panic is a crash.
- Do not leak SQL strings, stack traces, provider tokens, or full paths to the frontend.
- Distinguish "file moved/deleted" from "archive corrupt" from "permission denied"; these need
  different UX.
- Jobs report per-item failures without failing the whole batch.

### 13.2 Frontend handling

- One typed `invoke` wrapper normalises errors, logs them, and maps `code` to user actions.
- Route-level error boundaries; a reader crash must not blank the library.
- Non-fatal errors become dismissible toasts with a retry action.
- Fatal startup errors (database cannot open/migrate) show a dedicated recovery screen with
  "open data folder", "restore backup", and "quit".
- Never silently swallow errors. A failed progress write should retry and, if still failing,
  surface a warning before the user closes the app.

### 13.3 Logging and diagnostics

- Use `tracing` in Rust and a local file logger (for example `tauri-plugin-log`).
- Logs rotate and stay local. No telemetry by default.
- Include a diagnostics bundle action (logs, versions, DB schema version, library counts) that
  the user can inspect before sharing.
- Redact provider API keys and personal paths where a support bundle might be shared.

### 13.4 Crash safety

- SQLite transactions for multi-row changes.
- Atomic cache writes.
- Do not leave a scan job marked complete if it was cancelled mid-file.
- On startup, reconcile "indexing" files and "running" jobs from previous sessions.
- Treat unreadable files as data, not crashes: record the failure and continue.

---

## 14. Security and privacy

The current config has `csp: null` and no explicit threat model. That must change.

### 14.1 Content Security Policy

Set a restrictive production CSP. A sketch (not final; verify against Tauri/Vite needs):

```text
default-src 'self';
img-src 'self' comic: http://comic.localhost data: blob:;
style-src 'self' 'unsafe-inline';
script-src 'self';
connect-src 'self' ipc: http://ipc.localhost;
font-src 'self';
object-src 'none';
base-uri 'none';
frame-ancestors 'none';
```

Notes:

- `http://comic.localhost` is required for custom protocols on Windows.
- `'unsafe-inline'` for styles may be needed by the chosen styling solution; avoid it if
  possible. Avoid `'unsafe-eval'` in production.
- Dev mode needs Vite websocket/HTTP origins; use a separate dev CSP or Tauri's dev overrides.
- If remote provider images are displayed, they must be proxied/cached by Rust, not added to
  `img-src` as a broad `https:` wildcard.

### 14.2 Capabilities and plugins

- Grant only the permissions actually used. Remove `opener:default` unless a real feature needs
  it; if kept, restrict the allowed URLs.
- Do not add the shell plugin. Do not add broad filesystem scopes.
- Use the dialog plugin only for user-initiated folder/file selection; hand the result to Rust.
- If a future export feature writes files, use a dedicated Rust command with a user-selected
  destination, not a general filesystem capability.

### 14.3 Network policy

- No network request on first launch or on scan unless the user enables metadata fetching.
- Metadata requests are made by Rust, with timeouts, retry/backoff, provider rate limits, and a
  provider allowlist.
- API keys, if required, are stored in the OS keychain (or a dedicated secret plugin), not in
  SQLite or the frontend bundle.
- Provider terms of service and rate limits must be respected; scraping is out of scope.
- The application must remain fully functional if every provider is unreachable.

### 14.4 Untrusted content

- Archive limits and path sanitisation as described in §10.5.
- Metadata descriptions from providers must be rendered as plain text or sanitised HTML. Never
  inject provider HTML directly.
- `ComicInfo.xml` is untrusted XML: disable external entities and cap document size.
- SQL is always parameterised. Dynamic sort/filter clauses use an allowlist of column names.
- No `eval`, no dynamic script injection, no remote code loading.

### 14.5 Privacy

- Library contents, reading history, and spoiler preferences are sensitive personal data.
- Local-only by default; no analytics, crash upload, or cloud sync without explicit opt-in.
- Cover art and metadata caches are local. If a future sync feature is added, it must be
  end-to-end encrypted or clearly documented as not private.

---

## 15. Local-first and offline behaviour

| Capability | Offline behaviour |
| --- | --- |
| Open and read a comic | Fully functional |
| Browse library and series | Fully functional |
| Search | Fully functional over local index |
| Reading progress, bookmarks, collections | Fully functional |
| Reading orders and spoiler settings | Fully functional |
| Thumbnails on first run | Generated locally; slower until cached |
| Metadata lookup | Unavailable; existing data remains |
| Provider cover images not yet cached | Unavailable; show placeholder |
| Character/arc data imported previously | Fully available |
| Multi-device sync (future) | Explicitly offline-tolerant; queues changes locally |

Offline-first rules:

- No core screen may block on a network request.
- Every provider-derived field is stored locally after first fetch, with a source and timestamp.
- The UI must show provenance and staleness for remote metadata; "last updated" is not optional.
- Sync, if added, must use local change logs and conflict handling, not a live server requirement.

---

## 16. Proposed dependency set

Nothing below is installed. Versions are indicative, not pinned.

| Purpose | Crate / package | Notes |
| --- | --- | --- |
| SQLite | `rusqlite` (`bundled`) | Embed SQLite; no system dependency |
| Migrations | `rusqlite_migration` or `refinery` | Embedded, transactional |
| Async/pool | `tokio` (already in Tauri's tree) | Use `spawn_blocking` for DB and IO |
| ZIP | `zip` | Random access, ZIP64 |
| 7z | `sevenz-rust` / libarchive | Evaluate; solid archives are awkward |
| RAR | `unrar` / libarchive | Licensing review required |
| Image decode | `image` | Also consider `zune-image` for speed |
| Fast hash | `blake3` or `xxhash-rust` | Full-content fingerprint |
| XML | `quick-xml` | Streaming, entity-safe |
| HTTP | `reqwest` (rustls) | Metadata only; no default cloud |
| Logging | `tracing`, `tracing-subscriber` | Local logs |
| Errors | `thiserror` | Domain-to-IPC mapping |
| Config/state | `serde`, `serde_json` | Already present |
| Frontend query | `@tanstack/react-query` | Cache, pagination, invalidation |
| Frontend router | React Router or TanStack Router | Choose before feature work |
| Client state | `zustand` (small stores) | Reader session, UI |
| Virtualisation | `@tanstack/react-virtual` | Grids, strips, lists |
| Type bridge | `specta`/`tauri-specta` or `ts-rs` | Decide via spike |
| Tests | Vitest + Testing Library; Rust `#[test]` + `cargo nextest` | Not present yet |
| Lint/format | ESLint + Prettier; `rustfmt` + `clippy` | Not present yet |

Every dependency is a supply-chain and maintenance cost. Prefer the standard library or an
existing dependency over a new one, and justify image/archive/PDF crates explicitly.

---

## 17. Decisions versus recommendations

### Decisions taken in this document (need approval)

| ID | Decision | Rationale |
| --- | --- | --- |
| D1 | Model library files and catalog issues separately; files are readable without metadata | Prevents a hard dependency on matching and supports loose files |
| D2 | SQLite + `rusqlite` (bundled), WAL, single writer, read pool | Deterministic offline store without an async ORM learning curve |
| D3 | Serve page/thumbnail bytes through a `comic://` custom protocol, not IPC JSON | Performance, caching, security boundary |
| D4 | All archive, database, filesystem, and network work stays in Rust | Matches Tauri's model and the stated design requirements |
| D5 | CBR/RAR is not a launch requirement unless licensing is approved | Redistribution risk and platform packaging complexity |
| D6 | Spoiler filtering is enforced in Rust/SQL, keyed by a reader profile, not in React | A frontend-only filter leaks data through search, summaries, and APIs |
| D7 | Generated TS bindings for IPC DTOs (tool pending spike) | Avoids silent contract drift |
| D8 | Reading orders are first-class, ordered lists of stable issue references with optional franchise/era metadata | Orders must span publishers and series |
| D9 | Files are indexed in place; the app does not import or copy by default | Respects user storage and avoids duplicate data |
| D10 | Migrations are forward-only with pre-migration backups | Local data is not recoverable from a server |
| D11 | Production CSP is restrictive and capabilities are minimal | Current template posture is unsafe for untrusted archives |
| D12 | No background/watched scanning in the first release; explicit rescan | Simpler, predictable, cross-platform |
| D13 | Thumbnails and transcodes are content-addressed and disposable | Safe eviction and regeneration |
| D14 | Zoom uses CSS transforms with bounded decoded pages; no tiling in CORE | Tiling is a large subsystem; set expectations that CSS zoom adds no detail |

### Recommendations still to evaluate

| ID | Recommendation | Why it is not decided |
| --- | --- | --- |
| R1 | TanStack Query for server state | Strong fit, but adds a dependency and a learning curve; a small custom cache is possible |
| R2 | Zustand for ephemeral reader state | Redux Toolkit or plain reducers are viable |
| R3 | `specta` over `ts-rs` | Needs a spike against Tauri 2 command/event shapes |
| R4 | React Router vs TanStack Router | Both work; pick for team familiarity and typed routes |
| R5 | Deep-zoom tiling for very large scans | Significant subsystem; defer until profiling shows it is necessary |
| R6 | CB7 and PDF support in the first release | Increases parser surface and packaging complexity |
| R7 | Filesystem watching | Nice UX, unreliable on some filesystems; reconcile-based rescan is safer |
| R8 | Multi-profile support in v1 | The schema should support it; the UI can ship with a single profile |
| R9 | Import/export format for reading orders | JSON is easy; a documented schema and conflict rules need design |
| R10 | Full-text search ranking and filter set | Needs UX prototyping against a real library |
| R11 | Cover image extraction heuristics | Some archives have no cover; placeholder and manual override needed |
| R12 | Optional encrypted sync | Large design and trust surface; out of scope until requested |
| R13 | Fractional `position REAL` with gap-based insertion for order items | Simple; needs renormalisation after repeated inserts in the same gap |
| R14 | Keep `timelines` separate from `reading_orders` | Different progress and generation semantics; merge only if they converge |
| R15 | Spoiler v1 uses explicit read marks rather than a numeric frontier | Correct and indexable; cannot represent partial-issue or order-relative position |

---

## 18. Known risks and technical problems

1. **Archive parsing is an attack surface.** ZIP/RAR/7z parsers and image decoders have a history
   of memory-safety and decompression-bomb issues. Rust helps but does not eliminate logic bugs.
   Limits and fuzzing are required, not optional.
2. **RAR licensing may block a launch feature.** Do not promise CBR support before legal review.
3. **WebView inconsistency.** Linux/WebKitGTK will behave differently from WebView2. Image
   formats, GPU acceleration, and large-image memory need per-platform testing.
4. **SQLite write contention.** A naive single-mutex design will stutter during scans. Separate
   writer and reader paths from the start.
5. **CSS zoom is not real zoom.** Very large scans will look soft. Tiling is the only real fix
   and is expensive. Set expectations.
6. **Chronological reading orders are a data problem, not a UI problem.** Ordering across
   publishers requires stable issue identity, curated order data, and a conflict policy when
   orders disagree. Do not assume automated chronology is reliable.
7. **Metadata matching is probabilistic.** Wrong matches corrupt reading progress and spoilers.
   Matching must be reviewable, reversible, and never silently overwrite user edits.
8. **Spoiler protection is easy to leak.** Search snippets, "next issue" previews, character
   lists, and arc summaries can all reveal future events. Filtering must be centralised, not
   per-component.
9. **Spoiler protection is hard to define.** "Read up to issue X" is simple; "read up to a point
   inside an issue" and "read this order, which includes only part of a series" are much harder.
   The data model must support issue-level and order-position-level progress.
10. **Two sources of truth for progress.** File-level position and issue-level progress can
    disagree if a file maps to multiple issues or an issue has multiple files. Define one
    canonical mapping.
11. **Thumbnail cache growth.** A large library can generate tens of thousands of files. Eviction,
    size accounting, and startup cleanup must exist early.
12. **Migration failures can lose a library index.** Backups before migration are mandatory; a
    failed migration must leave the previous database intact.
13. **Cross-platform paths.** Windows drive letters, UNC paths, macOS case-insensitivity, NFD
    Unicode normalisation, and Linux symlinks all break naive path comparisons.
14. **Comic file diversity.** Filenames, folder conventions, and metadata quality vary wildly.
    Any importer based only on filename parsing will fail on a large fraction of real libraries.
15. **Test data is hard.** A meaningful test suite needs small, redistributable sample archives
    and synthetic fixtures; do not commit copyrighted comics.

---

## 19. Testing strategy (planned, not implemented)

- **Rust unit tests:** natural sort, path validation, archive-limit checks, filename parser,
  migration runner, spoiler SQL predicates.
- **Rust integration tests:** temporary SQLite databases, synthetic ZIP fixtures, protocol
  handler responses, scanner idempotency.
- **Frontend unit tests:** layout strategy math (spread pairing, fit modes), command registry
  scope resolution, progress throttle logic.
- **Frontend component tests:** reader controls, bookshelf virtualisation, error boundaries.
- **End-to-end (later):** Tauri WebDriver or Playwright against a built app, with a temp library.
- **Performance budgets:** cold start, scan of 1,000 files, open time for a 300-page CBZ,
  scroll smoothness in continuous mode, memory after 100 page turns.
- **Fuzzing:** archive and image parsers are good candidates once they are isolated.

---

## 20. Summary of the architecture

The application is a single Tauri process with a strict split:

- **React** owns presentation, interaction, routing, and ephemeral reader state.
- **Rust** owns the database, archives, image processing, filesystem access, metadata fetching,
  background jobs, and business rules including spoiler filtering and reading-order resolution.
- **SQLite** is the durable local catalog. **Files stay on disk.** **Images travel over a custom
  protocol**, never through JSON.
- The catalog is designed around two independent concepts: **files** (what exists on disk) and
  **issues** (what the file is). This keeps the library readable with zero metadata and makes
  reading orders, characters, arcs, and timelines additive rather than foundational.

The next document, `DATA_MODEL.md`, turns this into a concrete schema and explains how reading
orders, spoiler protection, and character data are represented without a later rewrite.

