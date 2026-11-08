# Session handover

Read `AGENTS.md` first (commands, layout, conventions, streaming rules). Then `design.md`
is the spec; this file is only state.

## Verified green

- `cd src-tauri && cargo test` -> 21 passed (`tests/core.rs`)
- `npm test` -> 20 passed (Vitest)
- `npm run typecheck`, `npm run lint`, `npm run clippy`, `npm run build` -> clean
- `cargo build --release` succeeds; runs against a seeded 25-issue library
- Windows hardening done: dunce paths, forward-slash storage, NSIS/MSI bundle targets,
  bundled fonts (Latin subset only, ~144 KB total), tightened CSP

## Implemented

Backend (`src-tauri/src`): `db` (schema **v4**, migrations, backup), `library` (CBZ/ZIP/CBR/
PDF/image-folder import, duplicates, rescan, cancellable + progress-reporting scans), `archive`,
`pdf` (pure-Rust image-page extractor), `metadata`, `repo` (queries, progress, orders,
collections, search incl. bookmark notes, stats, history, missing-issue detection, per-series/
per-file reading direction, variants), `protocol` (`comic://`), `commands` (56 registered IPC),
`models`.

Frontend (`src`): Library, Series + detail (reading direction, cover picker, missing-issue
panel, variant grouping), Reader (single/double/vertical/horizontal, RTL, zoom/pan, transitions,
bookmarks with notes, per-file direction override, spread offset), Collections + detail
(drag-and-drop reorder), Reading orders + detail (follow mode, auto-advance, drag-and-drop),
Search, History, Stats, Settings, command palette, toasts, import progress + cancel.

## Schema notes (v3/v4)

- v3: `series.reading_direction`, `missing_issues(series_id, number, reason)`.
- v4: `issues.variant_of`, `issues.edition` (alternate covers / special editions).
- Reading direction resolution order in `repo::reader_payload`: file override
  (`file_preferences.reading_direction`) -> series (`series.reading_direction`) ->
  global setting.
- Missing issues are computed on demand for numeric numbers only; spans over 500 are ignored
  to avoid a stray issue number generating thousands of rows.

## Remaining work, in priority order

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
3. **Alternate cover comparison.** Variants are linked and grouped, but spec 3.4's side-by-side
   page/cover comparison view is still unbuilt.
4. **Import metadata correction during import.** Spec 3.2 asks for editing metadata before the
   import commits; today correction happens after import from the library.
5. **Reading session resume.** Sessions are recorded; resuming an interrupted session (as
   opposed to resuming page progress) is not modelled.
6. **Settings: per-series direction default shortcut.** Direction exists per series and per
   file; a bulk "set for all issues in series" action would speed up manga libraries.
7. **Import progress for rescan in the UI.** The backend emits events for both paths, but the
   library page's rescan button still shows only a spinner.

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
