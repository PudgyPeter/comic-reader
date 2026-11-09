# Session handover

Read `AGENTS.md` first (commands, layout, conventions, streaming rules). Then `design.md`
is the spec; this file is only state.

## Verified green

- `cd src-tauri && cargo test` -> 40 passed (`tests/core.rs`)
- Test library generator: `cargo run --example sonic_fixture -- <library-dir> <db-dir>`
  writes 26 synthetic CBZ issues (gaps at 7/12/15, one annual, one Cover B variant,
  a companion run with one gap), imports them, and seeds progress, a bookmark,
  a collection and a publication order.
- `npm test` -> 34 passed (Vitest)
- `npm run typecheck`, `npm run lint`, `npm run clippy`, `npm run build` -> clean
- `cargo build --release` succeeds; runs against a seeded 25-issue library
- Windows hardening done: dunce paths, forward-slash storage, NSIS/MSI bundle targets,
  bundled fonts (Latin subset only, ~144 KB total), tightened CSP
- Metadata lock (schema v7), smart collections (schema v8), unmatched files + match-to-issue
  (schema v9); all with UI and tests

## Implemented

Backend (`src-tauri/src`): `db` (schema **v9**, migrations, backup), `library` (CBZ/ZIP/CBR/
PDF/image-folder import, duplicates, rescan, cancellable + progress-reporting scans), `archive`,
`pdf` (pure-Rust image-page extractor), `metadata`, `repo` (queries, progress, orders,
collections, search incl. bookmark notes, stats, history, missing-issue detection, per-series/
per-file reading direction, variants), `protocol` (`comic://`), `commands` (71 registered IPC),
`models`.

Frontend (`src`): Library, Unmatched files (list + match-to-issue, reader for loose files),
Series + detail (reading direction, cover picker, missing-issue
panel, variant grouping), Reader (single/double/vertical/horizontal, RTL, zoom/pan, transitions,
bookmarks with notes, per-file direction override, spread offset), Collections + detail
(drag-and-drop reorder), Reading orders + detail (follow mode, auto-advance, drag-and-drop),
Search, History, Stats, Settings, command palette, toasts, import progress + cancel.

## Test library

`cargo run --example sonic_fixture -- <library-dir> <db-dir>` writes 26 synthetic CBZ issues
(Sonic the Hedgehog #1-20 minus 7/12/15, an Annual 2020, a Cover B variant linked to #3, and
Sonic Universe #1-8 minus 4), imports them into the given database, and seeds progress, a
bookmark, a collection and a publication order. All pages are generated shapes; nothing is
copyrighted. Delete the DB first if it already holds test data, or the fixture will merge
series and duplicate numbering.

## Schema notes (v3/v4/v5)

- v3: `series.reading_direction`, `missing_issues(series_id, number, reason)`.
- v4: `issues.variant_of`, `issues.edition` (alternate covers / special editions).
- v5: `progress.scroll_offset` (continuous-mode resume), `reading_sessions.last_page`,
  `reading_sessions.status` (`open`/`closed`), `reading_sessions.resumed_at`.
  A session is inserted `open` on reader mount and closed on unmount; open rows surface as
  interrupted sessions on History and are never listed as finished history.
  `repo::save_progress` delegates to `repo::save_reading_position`, which takes an optional
  page-relative offset; `None` keeps the stored offset so paged modes do not erase it.
- Reading direction resolution order in `repo::reader_payload`: file override
  (`file_preferences.reading_direction`) -> series (`series.reading_direction`) ->
  global setting.
- Missing issues are computed on demand for numeric numbers only; spans over 500 are ignored
  to avoid a stray issue number generating thousands of rows.

## Metadata lock (CORE-7/CORE-9) — schema v7

- v7 adds `issues.metadata_locked` and `series.metadata_locked` (0/1, default 0).
- `IssuePatch.metadataLocked` / `SeriesPatch.metadataLocked` are optional tri-state:
  omitted keeps the stored value. Editing any metadata field through `repo::update_issue`
  or `repo::update_series` sets the lock automatically; passing `false` unlocks explicitly.
- `repo::bulk_update` skips locked issues unless the patch changes the lock itself, and
  `BulkEditor` reports how many were skipped.
- `repo::refresh_issue_metadata(conn, id, force)` re-reads ComicInfo.xml. Automatic callers
  must pass `force = false`, which returns `false` for a locked issue; the IssueEditor
  "Refresh from file" button calls `refresh_issue_metadata` with `force` so an explicit user
  refresh works. Rescans only skip already-indexed relative paths, so they never rewrote
  metadata in the first place — the lock is the guard for future provider refresh paths.
- Series universe/publisher propagation into issues now excludes locked rows.
- UI: `IssueEditor` has the lock checkbox + "Refresh from file"; `SeriesDetailPage` shows a
  "Locked" chip and a lock glyph on locked issue rows.
- Test: `metadata_lock_protects_manual_edits_from_refresh_and_bulk_edits` covers edit→lock,
  skipped automatic refresh, skipped bulk edit, manual unlock+force refresh, series locking.

## Smart collections (CORE-13) — schema v8

- v8 adds `collections.is_smart`, `collections.smart_rules` (JSON) and
  `collections.sort_mode` (`manual|title|series|added_desc|added_asc|release_date`).
- Rule model (`models::SmartRules`) is `{ all: [rule], any: [rule] }`; groups are ANDed, so
  `(all…) AND (any…)`. A rule is `{ field, op, value }`.
- **Compilation is allowlisted, not an expression language.** `repo::smart_rule_column` maps
  15 fixed field names to columns; `compile_smart_rule` maps 10 fixed operators (`contains`,
  `eq`, `ne`, `startsWith`, `gt`, `gte`, `lt`, `lte`, `isSet`, `isEmpty`) to fixed SQL, and
  every value is a bound `?` parameter. Unknown fields or operators are rejected at save time
  with `AppError::invalid`. LIKE values escape `%`, `_` and `\` and use `ESCAPE '\\'`.
  Numeric fields (`year`, `pages`, `lastPage`) are cast and validated as numbers.
- An empty rule set compiles to `0 = 1` so a half-built smart collection matches nothing.
- Membership is computed on read, so smart collections never store `collection_items`.
  `repo::collections` recomputes smart counts; `collection_detail` and the `collection_id`
  filter in `search_filtered` reuse the same compiled WHERE (detail adds a whitelisted
  ORDER BY from `sort_mode`).
- `repo::add_to_collection` rejects smart collections, and `AddToCollectionMenu` disables them.
- UI: `CollectionsPage` has "New smart collection" (`SmartCollectionForm`), the edit modal
  exposes sort order + `SmartRulesEditor`, and `CollectionDetailPage` hides reorder/remove and
  labels rule-driven membership.
- Mock harness: `?mock&smart` opens the builder. The mock bridge re-implements rule matching in
  `applySmartRules`/`smartRuleMatches` so the browser harness behaves like the backend.
- Test: `smart_collections_compile_typed_rules_and_reject_injection` covers AND/OR grouping,
  live rule changes, field/operator injection rejection, empty-rule behaviour, and the search
  collection filter.
- **Caveat:** there is no SQL-level guard preventing a manual `collection_items` insert for a
  smart collection; only the repo/UI paths enforce it.

## Large single-file archives (omnibus CBZ/CBR/PDF)

- A library root may now be **one archive file**, not just a folder
  (`library::normalize_root`). This is the path for very large collections, e.g. a
  1.6 GB CBZ holding the whole Sonic run. `scan_candidate_paths` returns the file itself,
  and `relative_to_root` uses its file name as the relative path.
- UI: the sidebar has "Add folder" **and "Add files"** (multi-select, CBZ/ZIP/CBR/PDF).
  `ImportReview` now takes a list of sources, previews each, and commits them one by one,
  merging the results — so a folder and a huge file can be reviewed in one pass.
- Import no longer holds every page in RAM. `archive::read_*` became `archive::scan_*` and
  returns `PageMeta { name, width, height }`: ZIP and folder pages are probed from a 64 KiB
  prefix, RAR uses a `PrefixWriter` that keeps only the prefix. Page bytes are read on demand
  through the existing `archive::page_bytes` path (already used by `comic://`), so memory use
  is flat regardless of archive size. PDFs are still fully decoded to discover page order.
- Tests: `single_archive_file_can_be_imported_as_its_own_root`,
  `non_archive_file_is_rejected_as_a_root`, `rescan_keeps_a_single_file_root_working`.

## Unmatched files + match to issue (CORE-7) — schema v9

- A file with no issue pointing at it (`NOT EXISTS … issues.file_id`) is an unmatched file.
  v9 adds `file_progress` and `file_bookmarks` so it stays readable and trackable on its own,
  per ARCHITECTURE.md §10.7.
- Import: `ImportOverride.keepUnmatched` indexes the file without creating an issue (checkbox in
  the import review screen). The file goes straight to the unmatched view.
- Reader: `/file/:fileId` reuses `ReaderPage` in file mode. All progress, bookmark, spread-offset
  and direction calls branch to file-scoped commands; no reading session is opened for a file.
- Matching never recreates the file. `repo::match_file_to_new_issue` inserts the issue with the
  existing `file_id`; `repo::match_file_to_existing_issue` sets `file_id` on a fileless issue.
  Both call `adopt_file_progress`, which copies `file_progress`/`file_bookmarks` into the
  issue-scoped `progress`/`bookmarks` tables and deletes the file-scoped rows in the same
  transaction, so resume position and bookmarks survive.
- `repo::unmatched_files` lists files with suggested series/number from the filename and their
  file-scoped progress. `repo::file_proposal` reuses `library::propose_metadata` (ComicInfo.xml
  then filename) for the match form. `repo::fileless_issues` lists issues with `file_id IS NULL`
  that an unmatched file can attach to.
- Fresh commands: `unmatched_files`, `file_proposal`, `apply_file_match`,
  `match_file_to_new_issue`, `match_file_to_existing_issue`, `fileless_issues`,
  `file_reader_payload`, `save_file_progress`, `file_bookmarks`, `add_file_bookmark`,
  `update_file_bookmark_note`, `delete_file_bookmark`, `set_file_spread_offset`,
  `set_file_direction_override`.
- UI: `/unmatched` page (nav "Unmatched") with per-row Read/Match; the match dialog can create a
  new issue from the proposal or attach to an existing fileless issue. `IssueCard.page_count`
  now coalesces to 0 so fileless issues display correctly.
- Test: `unmatched_files_stay_readable_and_match_without_losing_progress` covers keep-unmatched
  import, file-scoped read/progress/bookmark, match-to-new (file id reused, progress moved),
  double-match rejection, and match-to-existing-issue.

## Search (CORE-11) — FTS5

- Schema v6 adds `issue_fts` (FTS5, unicode61 with diacritic folding), kept in sync by
  triggers on `issues`, `series.title` and `bookmarks` (label + note), plus a one-time
  backfill so upgraded libraries are searchable immediately.
- `repo::search_filtered` supports free-text prefix queries (quoted tokens, so arbitrary
  input cannot inject FTS operators), publisher/series/universe/status/format/year-range/
  collection filters, four whitelisted sorts, keyset pagination via an opaque cursor, and
  snippets generated **after** spoiler filtering (`SearchResult.snippets`).
- Filter-only searches (no text) fall back to series order because there is no bm25 score.
- Recent searches are recorded (last 20, top 10 returned) with `clear_search_history`.
- `SearchPage` has a filter panel (publisher/series/status/format/year range/sort), recent
  search chips, inline snippets, and "Load more results" driven by `useInfiniteQuery`.

Known limitation: FTS triggers are SQLite triggers, so a direct `ALTER TABLE` or bulk copy
bypassing `issues` would desync the index; the backfill runs once per migration.

## Done in this pass (progress/session/import/compare)

- **Mark read up to page N** (CORE-8): `repo::mark_to_page` + `mark_issue_to_page` IPC; the
  issue editor has a "Mark read to page" control that clamps to the page count and completes
  the issue on the last page.
- **Continuous-mode scroll offset**: the reader stores the fractional position of the
  viewport centre inside the current page (debounced, clamped 0..1) and restores it on open
  and reload. Math in `src/lib/scroll.ts` with Vitest coverage.
- **Interrupted session resume**: reader mount opens a session, heartbeats every 30s and
  closes on unmount; History lists `open` rows in "Resume interrupted sessions" linking back
  with `?session=<id>&page=<n>`.
- **Alternate cover / edition comparison** (spec 3.4): `repo::issue_variants` and the
  `issueVariants` IPC; the reader has a page-synced side-by-side compare overlay with a
  version switcher, reachable from the toolbar and from variant rows in series detail
  (`?compare=<variantId>`).
- **Import-time metadata correction** (spec 3.2): `library::preview_import` proposes metadata
  without writing; `ImportReview` edits every ComicInfo/filename field and
  `library::commit_import` applies corrections during the import. Preview skips already
  indexed files and flags duplicates and single-page archives.
- **Brightness and gamma** (spec 2.4): persisted settings applied to reader pages via a
  CSS `brightness()` + SVG `feFunc*` gamma filter, controllable from Settings. No bitmap
  re-encoding, so it is instant and works in every reading mode.
- **High-resolution re-request on zoom** (CORE-3): the `comic://page/{file}/{page}/{width}`
  route re-encodes the page as a width-clamped JPEG and caches it under `cache/zoom/`.
  The reader requests a quantised 512px-bucket width once zoom passes 1.5x, so detail is
  real rather than a CSS-upscaled bitmap.
- Schema v5 with a v4→v5 migration test that also asserts existing progress survives.

Verified: `cargo test` 34 passed, `npm test` 34 passed, `npm run typecheck`, `npm run lint`,
`npm run clippy`, `npm run build` all clean.

## Remaining work, in priority order

0. **Hardening done this session.** App-level `ErrorBoundary`
   (`src/components/ErrorBoundary.tsx`) wraps `<App />`; the library grid/list now grows a page
   at a time via `src/hooks/useProgressiveList.ts` (sentinel + IntersectionObserver) instead of
   mounting every issue; component CSS moved into `@layer components` so Tailwind utilities
   (`w-auto`) actually win over `.input { width: 100% }`; the Most-read series bars scale by
   read count instead of total issues; generated publication orders skip alternate covers;
   coverless collection cards show one placeholder instead of four.
   Also: rescan now drives the same sidebar progress panel as folder import (shared
   `src/store/importActivity.ts`); series pages gained "Mark all read"/"Mark all unread" and a
   bulk "Direction for all issues" select (`repo::set_series_status`,
   `repo::set_series_direction`); the reader bottom bar has a numeric jump-to-page input;
   Settings shows shortcut conflicts and an expandable cheat sheet
   (`src/lib/shortcuts.ts`, 8 Vitest cases).
1. **Light-theme visual pass — verified, keep as regression check.** All 11 routes (`/`, series
   list + detail, collections list + detail, orders list + detail, search, history, stats,
   settings) were captured in light + midnight via the mock harness. Fixed the last hardcoded
   dark surfaces: toast info tone, reader edge-button glyph, and the `Cover` placeholder
   (was `bg-ink-800`, now theme-token based — it showed as dark blocks on collection cards).
   Re-screenshot after any layout change.
2. **Missing-issue UI on the library/series list — done.** `SeriesSummary.missingCount`
   (computed in `repo::fill_missing_counts` via the same gap logic as `series_detail`) is
   rendered as a warning chip on series cards and the library shelf. Regression: re-check the
   chip in light + midnight after card layout changes.
3. **Alternate cover comparison — done.** Keep the overlay in the light/midnight screenshot
   pass.
4. **Import metadata correction + metadata lock — done.** Manual issue/series edits lock
   themselves and survive bulk edits and automatic refreshes. Next metadata gap is provider
   adapters (CORE-9) behind one trait; those must call `refresh_issue_metadata` with
   `force = false`.
5. **Smart collections — done (schema v8).** Typed rules, allowlisted compiler, rule-driven
   counts/detail/search filter and a UI builder. Next collection gap is orphan cleanup if the
   polymorphic `collection_items` model ever grows; today it only stores issues with real FKs.
5b. **Unmatched files + match to issue — done (schema v9).** `/unmatched` lists loose files,
   `/file/:id` reads them, and matching reuses the file row so progress/bookmarks carry over.
   Next step is match *proposals* (DATA_MODEL.md §12.5: confidence + reasons + reject/suppress)
   once provider adapters exist; today the match dialog is manual-only.
5. **Reading session resume — done for one device.** Cross-device/sync resume is Stage 3.
6. **Mark up to a page — done per issue.** Bulk "mark up to page N" across a selection is not
   built. Per-profile spoilers (CORE-14) and provider adapters (CORE-9) remain the largest
   open items.
7. **Continuous-mode scroll offset — done.** Stored as a page fraction, so it is
   resolution-independent.

Deferred by design (Stage 2/3): panel detection, characters/relationships/locations/arcs/
timeline, appearance tracking, sync, recommendations, full comparison view.

## How to verify UI without a working webview

The Linux sandbox cannot render the Tauri webview faithfully (WebKit network-process crash +
software-compositing striping). Use the mock harness instead:

- `npm run dev`, then open `http://localhost:1420/?mock#/`
- Options: `?mock&mode=double|vertical|horizontal&dir=rtl&offset=1` combined with a reader route
- Theme: append `&theme=light` or `&theme=midnight` (mock-only override; it also updates the
  stored mock settings so the Settings theme buttons highlight the rendered theme)
- Regenerate fixtures after schema/data changes (run from `src-tauri`):
  ```bash
  cargo run --example export       -- ~/.local/share/com.comicreader.app ../src/mock/data.json
  cargo run --example export_pages -- ~/.local/share/com.comicreader.app ../public/mock-pages
  ```
  `export_pages` writes `{fileId}/{index}.{ext}` for every readable issue and reproduces the
  checked-out fixture byte-for-byte. If the database is empty, seed it first with
  `cargo run --example seed -- <db-dir> <library-dir>`, then
  `cargo run --example organise -- <db-dir>`.
- Screenshot helper:
  ```bash
  google-chrome-stable --headless=new --no-sandbox --disable-gpu --hide-scrollbars \
    --screenshot=out.png --window-size=1440,900 --virtual-time-budget=9000 \
    "http://localhost:1420/?mock&theme=light#/"
  ```
  Add `--dump-dom` to assert themed classes instead of eyeballing colour.

Note: covers looked striped in WebKit screenshots earlier; that was verified to be a sandbox
compositing bug (same bytes render fine in Chrome), not an app defect.

## Gotchas

- PDF support is image-per-page only (DCTDecode/JPXDecode/FlateDecode). Text PDFs return a
  clear error — do not claim PDF rendering.
- `parse_value` in `pdf.rs` must keep returning an absolute offset including leading
  whitespace; that bug silently broke stream detection once.
- Windows: store paths via `library::to_stored_path` (forward slashes) and compare with
  `paths_equal` (case-insensitive on Windows). Do not use raw `canonicalize` (verbatim prefix).
- `repo::apply_spoilers` is the only place spoiler filtering happens; new issue-surfacing
  queries must go through it.
- `library::DISPLAY_FIELDS` and `map_issue_offset` must stay in sync. Column order is
  positional; adding a field means updating both plus every `base + N` offset.
- Import cancellation returns `AppError::Cancelled` (`kind: "cancelled"` on the wire); the
  frontend checks `error.name === "cancelled"` after `normalizeError`. Do not match on the
  message string.
- Variant links are one level deep (`issues.variant_of`); `update_issue` rejects cycles and an
  empty string clears the link.
- Unmatched files are files with no `issues.file_id` row. File-scoped progress/bookmarks live in
  `file_progress`/`file_bookmarks`; only `repo::adopt_file_progress` may move them, and it must
  stay inside the match transaction. Anything that creates an issue for a file must reuse the
  existing `file_id`, never re-import.
- `archive::scan_*` returns metadata only. Never reintroduce a variant that buffers every page;
  it is what kept multi-GB archives from importing. `page_bytes` is the only page-data entry.
- A root may be a single archive file. Any code that does `strip_prefix(root)` or walks the root
  must handle `root.is_file()` (use `library::relative_to_root`).
- `SCHEMA_VERSION` is 9; migration tests must drop `file_progress`/`file_bookmarks` when
  simulating pre-v9 databases, and the v4 migration test already does.

## Windows executable

Verified working artifacts (built from Linux via `cargo-xwin`, no Windows host needed):

- `dist-windows/Panel.exe` — portable single-file exe. Double-click to run; no installer and
  no VC++ redistributable (imports only UCRT API sets that ship with Windows 10/11).
- `dist-windows/Panel_0.1.0_x64-setup.exe` — NSIS per-user installer (4.5 MB).

Smoke-tested under Wine: the PE launches and reaches window creation, which fails only
because Wine has no WebView2/display. It is a `Subsystem 2 (Windows GUI)` PE32+ with the
icon/version resource present.

Rebuild command (from repo root):

```bash
npm run build
source scripts/windows-build-env.sh   # clang-cl, llvm-lib, lld-link, LIB/RCFLAGS
npx tauri build --target x86_64-pc-windows-msvc
```

Toolchain prerequisites on a Linux host (one-time):

```bash
rustup target add x86_64-pc-windows-msvc
cargo install cargo-xwin --locked
sudo apt-get install -y clang-18 lld-18 llvm-18 nsis wine64
# ~/.local/bin needs clang-cl and llvm-lib wrappers around llvm-18, plus a
# makensis.exe wrapper so tauri's NSIS bundler finds a "Windows" makensis:
#   printf '#!/bin/sh\nexec /usr/lib/llvm-18/bin/clang --driver-mode=cl "$@"\n' > ~/.local/bin/clang-cl
#   printf '#!/bin/sh\nexec /usr/lib/llvm-18/bin/llvm-lib "$@"\n'                > ~/.local/bin/llvm-lib
#   cp ~/.local/bin/llvm-lib ~/.local/bin/lib.exe
#   printf '#!/bin/sh\nexec /usr/bin/makensis "$@"\n'                            > ~/.local/bin/makensis.exe
#   chmod +x ~/.local/bin/{clang-cl,llvm-lib,lib.exe,makensis.exe}
```

`src-tauri/tauri.windows.conf.json` is auto-merged on Windows targets and narrows the bundle
list to `nsis`/`msi` so a Windows host build does not try to emit deb/appimage/dmg.

Caveats:

- The exe requires WebView2, which ships with Windows 11 and current Windows 10. The NSIS
  installer uses `downloadBootstrapper`, so a machine without it downloads the runtime.
- Builds are unsigned (`sign_command` not configured); Windows SmartScreen will warn on first
  run until the artifact is code-signed.
- MSI was not produced here because WiX is a Windows-only tool; use the NSIS installer or
  build on a Windows host for an MSI.
