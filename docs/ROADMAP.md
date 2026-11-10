# Comic Reader — Development Roadmap

> **Status:** CORE (Stage 1) largely implemented — 51 of 95 deliverables done, 32 partial,
> 12 not started (foundations F1–F10 plus CORE-1…CORE-14). Reading, library, series,
> collections, orders, search (FTS5 with filters), stats, smart collections, unmatched-file
> matching and omnibus splitting work offline. Metadata providers, per-profile spoilers, order
> import/export/sections and root relocation are not started. Every deliverable is marked
> inline; see [`HANDOVER.md`](./HANDOVER.md) for the live remaining-work list.
> **Companion documents:** [`ARCHITECTURE.md`](./ARCHITECTURE.md), [`DATA_MODEL.md`](./DATA_MODEL.md).
> **Scope:** A local-first desktop comic reader and library for Tauri 2, React, TypeScript, Rust,
> and SQLite.

---

## 1. Reading this roadmap

The three stages are **CORE → LATER → OPTIONAL**. They are not arbitrary buckets:

- **CORE** is the minimum product that is genuinely useful offline: read files, organise a library,
  track reading, find things, and avoid spoilers. A release that ships CORE must be usable by
  someone who never enables metadata providers or connects to the internet.
- **LATER** is features that depend on CORE data being accumulated and validated: panel
  detection, character graphs, timelines, locations, arc tracking, appearance tracking, and sync.
  They are not "small extras"; several are research-heavy.
- **OPTIONAL** is a dashboard built on data that CORE/LATER already collect. It must not be a
  prerequisite for anything.

Each stage starts with a **foundation** task. Those tasks are not features; they are the
scaffolding that prevents the features from becoming unmaintainable. They are listed separately
so they are not lost inside feature tickets.

### Stage exit criteria

A stage is not "done" because its features exist. It is done when:

- the features work **fully offline**;
- the data survives an application restart, a crash, and a migration;
- the UI remains responsive during scans and large reads;
- error paths (missing file, corrupt archive, database busy) have a tested user experience;
- the relevant parts of the design have an automated test.

---

### Status markers

Deliverables are marked inline so this file stays the single source of truth:

- `[x]` done and covered by a test or a manual check recorded in `HANDOVER.md`
- `[~]` partially done; the marker text says what is missing
- `[ ]` not started

## 2. Current state

| Item | State |
| --- | --- |
| Tauri 2 shell | Implemented; 95 IPC commands, `comic://` asset protocol |
| React UI | Library, Series, Reader, Unmatched, Collections, Orders, Search, History, Stats, Settings |
| Rust modules | `db`, `library`, `archive`, `pdf`, `metadata`, `toc` (OCR), `characters`, `web` (Wikipedia lookup), `repo`, `protocol`, `commands`, `models` |
| Database | Schema v11 (FTS5, metadata lock, smart collections, file-scoped progress, multi-issue files, series-scoped character roster) with versioned migrations and pre-migration backups |
| Archive support | CBZ, ZIP, CBR, image PDF, image folders; single-file roots and multi-issue (split) files |
| Tests / lint | 49 Rust integration tests (+2 behind `--ignored`: OCR split, live web lookup), 34 Vitest tests, clippy `-D warnings`, eslint. No CI pipeline, no Prettier/rustfmt config. |
| Security config | Tightened CSP, scoped capability set |
| Docs | `ARCHITECTURE.md`, `DATA_MODEL.md`, `HANDOVER.md`, this file |

The task tables below were written before implementation and are now annotated with
per-deliverable status. Where a note and the marker disagree, the marker and its bold note are
authoritative. Use [`HANDOVER.md`](./HANDOVER.md) for the live remaining-work list.

---

## 3. CORE foundation (must precede feature work)

These are prerequisites, not features in the product sense.

| ID | Task | Why it is first |
| --- | --- | --- |
| F1 | [x] Rust module skeleton, `AppError`, `spawn_blocking` conventions | Every command depends on one error and concurrency model |
| F2 | [x] SQLite connection, pragmas, migration runner, backup-before-migrate | Data loss is unrecoverable without it |
| F3 | [~] IPC DTO conventions exist (handwritten `src/lib/types.ts` + `api.ts`); no generated bindings (`specta`/`ts-rs`) | Prevents silent contract drift from day one |
| F4 | [x] `comic://` custom protocol | Page delivery is architectural; retrofitting it is painful |
| F5 | [x] Frontend app shell: router, providers, error boundaries, feature folders | Giant components appear without this from the start |
| F6 | [x] Query cache + typed `invoke` wrapper + error mapping | Sets the data-fetching pattern before any screen exists |
| F7 | [x] Test harness: Vitest, Rust integration tests, synthetic CBZ fixtures. **`@testing-library/react` is installed but no test renders a component** — all 34 Vitest tests are pure logic (`spreads`, `scroll`, `shortcuts`, `reorder`, `utils`). | Untested archive and migration code is a liability |
| F8 | [~] ESLint + `clippy -D warnings` are enforced; **no** Prettier, no `rustfmt.toml`, no CI workflow | Cheap now, expensive later |
| F9 | [x] Restrictive CSP and minimal capabilities; remove unused `opener` grant | The template's security posture is not shippable |
| F10 | [~] Pre-migration DB backups only; no logging plugin, no diagnostics bundle, no crash-recovery report | Required to debug a desktop app you cannot inspect remotely |

**Sequencing note:** F1–F4 are backend-first; F5–F6 are frontend-first; F7–F10 are continuous.
Do not build a reader screen before F2/F4 exist, or you will rewrite it.

---

## 4. CORE features

### CORE-1 — Library import and file browsing

**Goal:** Point the app at folders, index files, browse them, and open any of them.

Deliverables:

- [x] Add/remove library roots through a native folder picker; paths stay in Rust.
  A root may also be a single large archive file (multi-GB omnibus CBZ/CBR/PDF), and one file
  can back several issues after an OCR-driven split.
- [~] Recursive scan with progress and cancellation. **Bounded**: `WalkDir` stops at
  `max_depth(6)` (silently — deeper files are never seen) and 50,000 candidates (that cap does
  emit a warning). **No resume**: a cancelled scan restarts from the beginning on the next run
  (it is incremental by relative path, so work is not lost, but it is not a resumable cursor).
- [x] File records with format, size, page count, and status; unmatched files visible
  (dedicated view + file reader, schema v9).
- [x] Manual rescan. **Missing files only**: a rescan flips `library_files.status` between
  `present` and `missing`; there is no separate `unreadable` state — an archive that exists but
  cannot be parsed surfaces as an import warning, and an already-indexed corrupt file only fails
  when opened.
- [ ] Root-aware paths: move a root, keep the library. **Only add/remove exist; there is no
  "relocate root" action.**
- [x] Background scanning that does not freeze the UI (`spawn_blocking` + progress events).
- [x] CBZ/ZIP and image-folder support (plus CBR/RAR and image PDF).

Originally deferred, since delivered: CBR/RAR and PDF. Still not in scope: **file watching**
(the library only changes on a manual rescan).

**Risks:** scan performance on network drives; case-insensitive path collisions; huge archives.
**Acceptance:** a 5,000-file library indexes incrementally and remains browsable during the scan.

### CORE-2 — Reader engine: multiple reading modes

**Goal:** A reader that handles single page, double-page spread, and continuous vertical strip.

Deliverables:

- [x] Reader state machine (`ARCHITECTURE.md` §12.2) and page manifest loading.
- [x] **Single page** with fit-width / fit-height / fit-page / actual size.
- [x] **Double-page spread** with cover-alone and odd/even alignment, per-file spread override,
  and RTL support (manga).
- [~] **Continuous vertical** (webtoon) mode. **Rendered by mapping every page, not a
  virtualiser**: `ReaderPage` mounts every page's `<img>` (eager within ±2, native
  `loading="lazy"` beyond). Bounded in pixels on screen, but the DOM is not windowed, so a
  900-page omnibus still mounts 900 nodes.
- [x] Spread overrides persisted in `file_preferences` (and per file for split issues).
- [~] Adjacent-page loading: pages within ±2 of the current page are marked `loading="eager"`,
  the rest `loading="lazy"`. **No explicit preload with cancellation**, and no in-flight request
  cancellation on fast turns.
- [ ] Clear loading/error states per page, not a blank screen. **Not built at page level**: the
  reader has a whole-issue `Spinner` and an issue-level failure panel, but the page `<img>` tags
  have **no `onLoad`/`onError` handlers**, so a page that fails to load renders as a blank box
  with no message.

**Criticisms to keep in view:**

- Spread pairing is a source of endless edge cases (double-page images, variant covers, inserts).
  Ship automatic pairing **plus** a manual override from day one.
- Continuous mode is the most memory-hungry mode; it needs virtualisation before it needs
  polish.
- Reading direction is a per-series/per-file setting, not a global toggle.

**Acceptance:** a 300-page CBZ opens in under a second after indexing; page turns are smooth;
memory stays bounded over 100 turns in each mode.

### CORE-3 — Advanced zooming

**Goal:** Usable zoom and pan on real scans.

Deliverables:

- [x] Zoom to point (cursor anchor) — `useReaderZoom.zoomAt` anchors on the pointer.
- [x] Pan with pointer/trackpad, clamped at page edges.
- [x] Fit modes and a "reset zoom" command.
- [x] Keyboard zoom shortcuts and a visible zoom indicator (percentage in the toolbar).
- [~] Bounded decoded image size: only the width-scaled route is bounded
  (`page_bytes_scaled` clamps 320–8192; `thumbnail` likewise). The unzoomed
  `comic://page/{file}/{page}` route still returns the **full** entry up to the 128 MiB
  `MAX_ENTRY_BYTES` cap, so a normal page turn decodes at native size.
- [x] High-resolution re-request for zoomed views (`hiResWidth`), cached on disk by
  `comic://` under the cover/page cache path.

**Explicit non-goal:** deep-zoom tiling. This is a decision (D14). If real scans prove soft,
schedule tiling as a separate research task with a clear memory budget, not as a "zoom improvement".

**Acceptance:** zooming a 4,000×6,000 page to 400% shows real additional detail (because a
higher-resolution variant was requested), not a blurred CSS-scaled image.

### CORE-4 — Page transitions

**Goal:** Pleasant, fast page changes without interfering with reading.

Deliverables:

- [x] Instant / slide / fade / page-turn transitions.
- [x] Respects `prefers-reduced-motion` (global CSS rule collapses durations).
- [~] No animation on rapid consecutive turns: transitions are CSS-only with a short duration,
  but there is no explicit "input faster than transition" bypass.
- [x] Transition duration and behaviour configurable, and disableable ("instant").
- [~] No per-page loading state at all, so there is also no preload gate: a transition can start,
  and complete, before the page has even decoded.

**Criticism:** transitions are a common source of perceived slowness in comic readers. Treat
"instant" as the default and transitions as an option. Do not build a physics engine.

### CORE-5 — Reading customisation

**Goal:** Make the reader comfortable across devices and preferences.

Deliverables:

- [~] Global, per-series and per-file resolution. Implemented for **reading direction** (global →
  series → file) and **spread offset** (per file). Reading mode, fit, background, brightness and
  gamma are **global only**; there is no per-series override for them.
- [~] Reading mode, direction, fit, spread, background, brightness and gamma all exist.
  **Toolbar auto-hide is not implemented**; the toolbar toggles manually (`showInterface`).
- [x] Page number/percentage display and a jump-to-page control.
- [x] Full-screen mode. Distraction-free is approximated by hiding the chrome, not a separate mode.
- [~] Controls are keyboard-driven via the command registry; **no focus trap in dialogs** (Tab
  leaves the modal) and accessibility labels are sparse — 58 `aria-label` occurrences in 14
  files, concentrated in the reader and library; the collections, orders, unmatched and history
  screens are mostly unlabelled.

**Criticism:** preference layering is easy to implement badly. Use one resolver function with
tests, not scattered `series?.mode ?? global.mode ?? 'single'` expressions.

### CORE-6 — Visual bookshelf

**Goal:** A library that looks like a shelf and stays fast at thousands of items.

Deliverables:

- [~] Cover grid with **progressive rendering**, not true virtualisation: `useProgressiveList`
  grows the slice by 120 items via an IntersectionObserver sentinel. The DOM stays bounded in
  practice, but off-screen items remain mounted once loaded (no recycling). Cover aspect ratios
  are fixed at 2/3, so variable-ratio covers are letterboxed rather than laid out natively.
- [x] Cover extraction and cached thumbnails on disk, with an initials placeholder for missing
  or failed covers.
- [x] Sort (title, added, release date, last read) and filters (read status, publisher,
  universe).
- [~] **Format filter is missing.** `LibraryFilter` (`src/lib/types.ts`) and
  `repo::library_snapshot` support search/status/universe/publisher/series, not format;
  `LibraryPage` therefore has no format select, though `filter_options` already returns the
  formats used by Search. Adding it is a DTO + WHERE clause + one `<select>`.
- [x] Continue-reading row ("Continue your series") and "recently added" sort.
- [x] List view as an alternative for large libraries.
- [~] Empty and scanning states exist (empty `EmptyState`, plus the global import-activity
  panel in `AppShell`); **the shelf has no query-error state** — `empty` requires
  `snapshot.isSuccess`, so a failed `library` load renders neither the empty copy nor an error
  message, just the header over a blank area.

**Risks:** thumbnail generation storms on first open; layout jank from mixed aspect ratios.
**Acceptance:** 5,000 covers scroll at interactive frame rates; thumbnails generate in the
background and never block the grid.

### CORE-7 — Series organisation

**Goal:** Group issues into series and browse them sensibly.

Deliverables:

- [~] Series list and detail views with issues ordered by number. Ordering is numeric via
  `metadata::parse_number` → `issues.sort_number` (`ORDER BY i.sort_number`), so 9 sorts before
  10; **no test covers a series with more than 4 issues**, so the 9-vs-10 case is unverified.
- [ ] Manual issue reordering for specials/annuals that break numeric order. **Not built**;
  ordering is derived from `sort_number`, and editing the issue number is the only workaround.
- [x] Series metadata editing with metadata lock (schema v7): lock checkbox, auto-lock on edit,
  lock-aware bulk edits, forced refresh.
- [~] Publisher and universe values exist on series/issues: the library shelf filters by both,
  series cards display them, and series create/edit accepts them. **There is no grouping UI** —
  `SeriesPage` only has a text filter, and there is no franchise column, so neither publisher,
  universe, nor franchise can group the series list.
- [x] Variant handling as designed in `DATA_MODEL.md` (decision Q2): `variant_of`, variant count,
  cycle rejection, variant compare view in the reader.
- [x] Unmatched files view with a "match to issue" flow (schema v9): `/unmatched` lists loose
  files, `/file/:id` reads them with file-scoped progress/bookmarks, and matching reuses the file
  row so progress survives. Manual matching only; provider-driven match proposals are still open.
- [x] Multi-issue files (omnibus CBZ/CBR/PDF, schema v10): the contents page is OCR'd, chapters
  become issues with `page_offset`/`page_count`, and the reader shows only each issue's slice.
  Split is proposed, editable, and never applied without confirmation.

**Criticism:** automatic series grouping from filenames will be wrong often. Ship manual grouping
and correction before promising automation.

### CORE-8 — Reading tracker

**Goal:** Know what has been read and resume accurately.

Deliverables:

- [x] Resume at last page, including the fractional scroll offset in continuous modes.
- [x] Mark as read/unread works per issue and per series (`set_issue_status`,
  `set_series_status`), and "mark read up to page N" is exposed: `api.markIssueToPage` →
  `mark_issue_to_page` → `repo::mark_to_page`, wired to the "Mark read to page" form in
  `IssueEditor` (clamped to the page count, completes on the last page). **Bulk mark-to-page
  across a selection is not built** — the library bulk editor only patches metadata/status.
- [x] Read status derived from progress and reconciled to issue status.
- [x] Continue-reading row and a History page (completed list).
- [x] Progress survives rematching a file to an issue (schema v9 `adopt_file_progress`) and
  splitting a file (schema v10 rebases the containing range).
- [x] Throttled progress writes (400 ms debounce + flush on unmount), each a single atomic
  upsert (no explicit transaction needed because no statement sequence is involved).

**Criticism:** the file-vs-issue progress duality is the most likely source of user-visible bugs.
Write tests for the reconciliation before building UI on top.

### CORE-9 — Automatic metadata

**Goal:** Enrich the catalog from local embedded metadata first, remote providers second.

Deliverables:

- [x] `ComicInfo.xml` parsing (`quick-xml`, no DTD/entity expansion; the entry is skipped when
  over the 4 MiB `MAX_XML_BYTES` cap and the read is bounded by `.take()`).
- [~] Filename parser exists (`metadata::parse_filename`) and every import is previewed before
  it is applied. **No confidence scoring** — the preview shows the parse but does not rank it.
  `parse_filename`/`parse_number` have no unit tests of their own; coverage is the integration
  test `filename_parsing_handles_common_patterns`.
- [ ] Provider adapters behind one trait, user-enabled, offline-tolerant, rate-limited.
  **Not started.** No `Provider` trait, no HTTP client, no caching.
- [ ] Match proposals with reasons; accept/reject; never silently overwrite user edits.
  **Not started.** The matching that exists is manual (`/unmatched`).
- [ ] Provenance display ("this synopsis came from X on date Y"). **Not started.**
- [x] Manual metadata editing with lock (schema v7): editing, auto lock-on-save, lock-aware bulk
  edits, forced refresh from file.
- [ ] Cache and snapshot provider responses. **Not applicable yet** (no providers).

**Criticisms:**

- Provider APIs change, impose rate limits, or disappear. Treat every provider as optional and
  keep the adapter interface small.
- Automated matching will merge the wrong issue. Keep it reviewable and reversible.
- Scaffolding an HTTP client and a provider abstraction is easy; the hard part is match quality
  and conflict resolution. Budget for that, not for the HTTP code.

### CORE-10 — Chronological reading order

**Goal:** Curated and (optionally) generated orders that mix publishers, series, and formats.

Deliverables:

- [~] Create/edit/reorder an order of issues. **Unmatched files cannot be added**: order items
  are `issue_id` FKs only, so a loose file must be matched first.
- [ ] Sections/group labels ("Act I", "Before the war"). **Not built**; an order is a flat list.
- [ ] Optional/skippable items. **Not built.**
- [~] Order progress and next-unread item exist (OrderDetailPage "Up next"), but there is **no
  profile concept**, so "per-profile" is not implemented.
- [ ] Import/export of a versioned JSON order with UUIDs and unresolved placeholders.
  **Not built.** The only export is a whole-library JSON snapshot.
- [x] Orders are independent of series and publisher (flat list of issue references).
- [x] No canonical chronology is asserted; orders are curated and named by the user.

**Criticism:** "automatic chronological order" is not a solvable problem in general. In-universe
dates are inconsistent, flashbacks break naive sorting, and tie-ins are debatable. Ship curated
orders with optional generation from `story_start_date` as a draft. This is decision Q4.

### CORE-11 — Advanced search

**Goal:** Find things fast in a large local library.

Deliverables:

- [~] FTS5 search over titles, series, numbers, creators, publishers, descriptions and bookmark
  notes. **No character/arc/event text** (those schemas do not exist yet).
- [~] Filters: publisher, series, year range, read status and format are exposed in
  `SearchPage`; `universe` and `collectionId` exist in `SearchFilter`/`repo::search_filtered`
  but **have no UI control on the search page** (the collection filter is only exercised by the
  smart-collection test). **Character and arc filters are not** (no data).
- [x] Sort options and keyset pagination (`nextCursor`, stable cursor values).
- [x] Snippets are generated with SQLite `snippet()` and then **dropped entirely for any issue
  `apply_spoilers` marked protected** (`repo::search_filtered`), so a hidden synopsis cannot leak
  through an excerpt. Residual risk: an issue whose *description* matched still appears in the
  result list, which can itself hint at content.
- [x] Keyboard-first search (Ctrl/Cmd+K) with recent searches.

**Criticism:** spoiler-safe search is subtle. A character name in a result can spoil. Plan a
dedicated test suite with a profile whose frontier is mid-series.

### CORE-12 — Keyboard shortcuts

**Goal:** A reader that can be driven without a mouse.

Deliverables:

- [~] Central command registry (`src/lib/shortcuts.ts`) with user-remappable bindings stored in
  settings. **No scopes**: the reader and the shell both listen on `window`, so bindings are not
  context-scoped.
- [~] Defaults cover page turn, next/prev issue, fullscreen, zoom, bookmark and search.
  **No registry binding for jump-to-page**, and reading mode/fit are hardcoded keys (`D` cycles
  modes, `1`–`4` pick fit, `H` toggles chrome) that cannot be remapped and do not appear in the
  conflict checker.
- [x] Conflict detection plus a shortcut cheat sheet in Settings.
- [~] Focus handling exists in the reader, but dialogs have **no focus trap** — `Modal`
  (`src/components/ui.tsx`) focuses its container, restores focus on close and closes on Escape,
  but Tab still escapes the dialog.
- [~] Most reading actions have a keyboard path; a few pointer-only affordances (spread-offset
  cycling via the toolbar) have no binding.

### CORE-13 — Custom collections

**Goal:** User-curated sets independent of reading orders.

Deliverables:

- [x] Create/rename/delete collections with covers and a description.
- [~] Add **series** and **issues** (series expands to its issues). **Arcs, characters, events
  and unmatched files cannot be added** — `collection_items` stores `issue_id` only, and there
  is no collection UI on the unmatched page.
- [~] Manual ordering works (`collection_items.position`), including a sort-mode selector for
  smart collections. **Per-item notes are not implemented** (no notes column, no UI); only a
  collection-level description exists.
- [x] Typed smart-collection rules compiled to SQL (schema v8): 15 allowlisted fields × 10
  operators, bound values, `LIKE` escaping, numeric validation, empty rules match nothing.
- [ ] Collection membership visible on entity detail pages. **Not implemented** — series and
  issue views do not list which collections contain them.

### CORE-14 — Spoiler protection

**Goal:** Do not reveal future story information to the active profile.

Deliverables:

- [~] Frontier model implemented as `spoiler_reveals` plus per-series/order progress, using the
  conservative v1 rule from `DATA_MODEL.md` §13.4. There is **no** separate
  `spoiler_read_marks` table; the frontier is derived from `progress` and `spoiler_reveals`.
- [ ] Per-profile spoiler mode; first-run choice of starting point (Q7). **Not built** — there is
  no profile concept, so spoiler protection is a single global setting.
- [~] Hidden: issue descriptions/synopses, future issues in lists, and search results are filtered
  by `repo::apply_spoilers`. **Character/arc/event hiding is not applicable yet** (no such data).
- [x] A visible indicator when content is hidden, with an explicit "reveal" action recorded in
  `spoiler_reveals` (IssueEditor).
- [~] The same shared filter is used by issue, series, search, order and collection queries.
  **Issue detail text nulling happens after the row is built**, so a raw DB read could bypass it.
- [~] Tests exist for `library_snapshot`, `issue`, `order_detail` and `search_filtered`.
  **`series_detail`, `collection_detail`, `reading_history` and `issue_variants` route through
  `apply_spoilers` but have no per-surface leak test.**

**Criticism:** this is the feature most likely to be declared done while still leaking. Define
the surfaces explicitly (issue detail, series detail, bookshelf hover, search, reading order
preview, character page, arc page, collections) and test each one. Do not rely on a `hidden` class.

---

## 5. LATER

These features depend on CORE data and should not begin until the corresponding CORE items are
stable. Several are research projects, not tickets.

### LATER-1 — Guided panel reading  — `[ ] not started`

Step through panels rather than pages: focus, zoom, and navigate panel-by-panel, with a page
overview.

**Why later:** it needs panel detection (LATER-2) and a whole new navigation model in the reader
state machine. It also interacts badly with double-page spreads and continuous mode.
**Decision needed:** is guided reading a separate mode or an overlay on the existing modes?

### LATER-2 — Panel detection  — `[ ] not started`

Automatic panel boundary detection for archives without panel metadata.

**Why later and why hard:** classical computer vision (edge/contour detection) is unreliable on
painted, full-bleed, or irregular layouts; ML models add a large dependency, platform variance,
and licensing questions. Expect manual correction to be necessary.
**Recommendation:** start with detection into an editable, per-page panel table; do not ship
detection as authoritative. Store results so failures are visible and fixable.

### LATER-3 — Character profiles  — `[~] series roster done, profiles partial`

Character pages with descriptions, aliases, first appearance, portraits, and a list of
appearances.

- [x] Series-scoped roster from `ComicInfo.xml` `<Characters>` (schema v11): `characters`,
  `character_aliases`, `character_appearances`; import records them inside the import
  transaction and `/series/:id` can rebuild the roster (`characters::sync_series`, idempotent,
  covered by `import_records_characters_from_comic_info` and
  `character_sync_is_idempotent_and_keeps_descriptions`).
- [x] Descriptions with explicit provenance (`comic` / `web` / `manual`). The local composer
  states series, first appearance and known-appearance count from library data only; a web
  summary (Wikipedia, `web.rs`) is fetched only on an explicit click and saved with its URL.
  Covered by `import_records_characters_from_comic_info` (comic source) and
  `web_lookup_returns_a_bounded_summary` (behind `--ignored`).
- [~] Appearance lists are spoiler-filtered through the shared `repo::apply_spoilers`
  (`character_appearances_are_spoiler_filtered`, which hides the title of an issue past the
  frontier). **Only the title is hidden** — the issue number and count stay visible by design.
- [ ] Portrait images, alias editing UI, and `is_death`/`is_first_appearance`/`appearance_type`
  flags. **Not built** — the v11 tables carry only name, description, aliases and issue links.
- [ ] Cross-series identity. Characters are unique per `(series_id, name)`, so the same
  character in two runs is two rows. **No universe-wide merge exists**; a merge UI is the
  prerequisite for relationships, timelines and arc tracking.

**Depends on:** the v11 tables, which now exist. The remaining work is additive (portraits,
alias UI, appearance types, cross-series merge), not a schema rewrite.
**Hard parts:** identity (same character across reboots/universes), data volume, spoiler-safe
appearance lists, and sourcing. Do not expect a complete character database from providers.

### LATER-4 — Relationship maps  — `[ ] not started`

A graph of character relationships with types and directions.

**Depends on:** `character_relationships`.
**Hard parts:** graph layout at scale, spoiler-filtered subgraphs (hide edges revealed later),
and representing changing relationships over time. A static graph over the whole run is
misleading; consider relationship states per era/arc.

### LATER-5 — Universe timeline  — `[ ] not started`

A visual timeline of events, arcs, and issues, with in-universe and publication views.

**Depends on:** `timelines`, `timeline_entries`, `events`, `arcs`.
**Hard parts:** precise dates are rare, concurrent storylines are common, and a timeline that
mixes publishers needs curatorship. Expect this to be a curated view, not an automatically
generated truth.

### LATER-6 — Story arc tracking  — `[ ] not started`

Arc progress across series and publishers, with "continue this arc" and arc-level spoiler state.

**Depends on:** `arcs`, `arc_issues`, reading progress.
**Hard parts:** arcs with optional tie-ins, reading order vs arc order, and marking an arc read
when its issues were read out of order.

### LATER-7 — Location database  — `[ ] not started`

Locations, appearances, and relationships (part of, located in, destroyed in).

**Depends on:** new `locations`, `location_appearances`, `location_relationships` tables (a purely
additive migration).
**Hard parts:** spoilers (a location's destruction is an event), naming collisions (multiple
"Gotham"s), and limited provider data. This is the least standardised domain; expect manual entry.

### LATER-8 — Character appearance tracker  — `[ ] not started`

A per-character timeline of appearances with filters and progress ("which appearances have I not
read?").

**Depends on:** `character_appearances`, spoiler read marks.
**Hard parts:** volume (thousands of rows per character), missing data, and the difference between
a cameo and a meaningful appearance. Needs pagination and an explicit data-quality indicator.

### LATER-9 — Multi-device synchronisation  — `[ ] not started`

Sync reading progress, bookmarks, collections, and reading orders across devices.

**Why later:** it changes the trust model. Options range from an encrypted file in a synced
folder to a self-hosted server to a hosted service. Each has different privacy, conflict, and
operational implications.
**Hard parts:** conflict resolution (two devices read different pages), stable IDs (UUIDs are
already planned), schema versioning across app versions, and secret management.
**Recommendation:** design the change-log and export format in CORE, implement no network sync
until there is a concrete user requirement. Local-first means sync must never be required.

---

## 6. OPTIONAL

### OPTIONAL-1 — Statistics dashboard  — `[~] basic stats done, no dashboard`

- [x] Reading time, pages read, issues completed, streaks, and per-series breakdowns
  (`repo::stats` + `StatsPage`).
- [ ] Yearly summary. **Not built.**
- [ ] Idle detection for reading time. **Not built** — session duration is wall clock between
  open and close, so leaving the reader open inflates it.
- [ ] Erase statistics without erasing progress. **Not built** (no stats-reset action).

**Why optional:** it is a nice-to-have that depends on `reading_sessions`, which is a private,
high-volume table. Collecting it without a dashboard is wasted storage; building a dashboard
without reliable collection produces misleading numbers.
**Prerequisites:** decide whether to collect sessions at all, define what counts as reading time
(idle detection), and provide a way to delete statistics without deleting progress.
**Privacy:** this is the most personal data in the app. Local only, exportable, erasable.

---

## 7. Proposed delivery sequence

The order below front-loads risk and keeps a usable product at every point.

```text
Phase 0   Foundations F1-F10                              ~ mostly done (F3, F8, F10 partial)
Phase 1   CORE-1 library import -> CORE-2 single page     done, plus CBR/PDF/folders
Phase 2   CORE-6 bookshelf + CORE-7 series organisation    done, plus unmatched + omnibus split
Phase 3   CORE-3 zoom + CORE-4 transitions + CORE-5 prefs  done (per-series prefs partial)
Phase 4   CORE-8 tracker + CORE-2 spread/continuous modes  done (bulk mark-to-page not built)
Phase 5   CORE-9 metadata                                  ComicInfo done; providers NOT started
Phase 6   CORE-11 search + CORE-12 shortcuts               done (scopes + some filters missing)
Phase 7   CORE-13 collections + CORE-10 reading orders     done (sections/import-export missing)
Phase 8   CORE-14 spoiler protection across all surfaces   partial (global only, not every surface)
Phase 9   Hardening                                        not started (a11y, diagnostics, packaging CI)
Phase 10  LATER features                                   started: series character roster only
```

Remaining CORE work, in dependency order: provider adapters + match proposals (CORE-9),
per-profile spoilers with per-surface leak tests (CORE-14), order import/export + sections
(CORE-10), bulk mark-to-page and manual issue ordering (CORE-8/CORE-7), missing bookshelf format
filter and query-error state (CORE-6), root relocation (CORE-1), per-page load/error states
(CORE-2), then the hardening pass (a11y/focus traps, diagnostics, CI).

**Why spoilers are phase 8, not phase 1:** the data model supports them from the start, but the
filter must cover surfaces that only exist later (search, orders, characters). Implementing the
policy early and then retrofitting every screen is how leaks happen. Build the shared filter in
phase 1 (F4/F6 territory) and enable the full policy once the surfaces exist.

**Criticism of this sequence:** it delays the reading-order feature, which is a headline
requirement. If reading orders are the primary motivator for the product, move CORE-10 to phase 4
and accept that spoiler filtering for orders arrives in phase 8. Do not build orders on a series-
coupled model "temporarily"; that is exactly the rewrite the design requirements forbid.

---

## 8. Cross-cutting work (every phase)

These are not features and must not be deferred to the end:

| Area | Requirement | State |
| --- | --- | --- |
| Offline | Every screen works with the network disabled; test this in CI or manually each phase | `[x]` Fonts bundled locally; the only runtime request is the user-initiated character summary lookup in `web.rs` (8 s timeout, 256 KiB cap, no caller waits on it) |
| Performance | Budgets for cold start, library scroll, reader open, and memory; measure, do not guess | `[~]` Import scans metadata only (`archive::scan_*`), so memory is flat by construction, but **no measurement is recorded** in `HANDOVER.md`; no automated budgets |
| Accessibility | Keyboard-first, focus rings, screen reader labels, reduced motion, sufficient contrast | `[~]` Keyboard + reduced motion done; sparse labels, no dialog focus trap |
| Error UX | Missing file, corrupt archive, DB busy, permission denied each have a designed state | `[~]` Reader/import have states; DB-busy and permission-denied are generic errors |
| Migrations | Every schema change has a migration, a backup step, and a test from the previous version | `[x]` v1→v11 with backup-before-migrate and a v4→current test |
| Security | New inputs (archives, XML, provider HTML) get limits and tests before release | `[~]` Entry size caps, safe XML, CSP; no fuzzing of the new OCR/archive paths |
| Diagnostics | Logs and a support bundle that do not leak paths or keys | `[ ]` Not started |
| Packaging | Installers and code signing for each target; test on a clean machine | `[~]` Windows NSIS + portable exe built; unsigned, not smoke-tested on Windows |

---

## 9. Testing by stage

| Stage | Must be tested | State |
| --- | --- | --- |
| Foundations | migration runner, backup/restore, protocol handler, error serialization | `[~]` migration runner covered by the v4→current test; **backup/restore, the `comic://` handler and error serialization have no test** |
| CORE-1 | scanner idempotency, cancellation, missing files, case collisions | `[~]` idempotency, missing files, single-file roots; cancellation only covers a pre-cancelled control plus the import loop guard, not a mid-scan cancel; no case-collision test |
| CORE-2/3 | spread pairing, fit modes, zoom anchor math, memory bounds | `[~]` spread pairing tested; zoom math and memory bounds are not |
| CORE-5 | preference resolution precedence | `[~]` only reading direction has a precedence test (file → series → global); mode/fit/background/gamma have no per-series resolution to test |
| CORE-6/7 | virtualisation, cover placeholders, numeric ordering, unmatched match keeps progress, omnibus split keeps progress | `[~]` match-keeps-progress and split-keeps-progress covered; **ordering, cover placeholders and the progressive list have no test** (no test imports a series with more than 4 issues, so 9-before-10 is unproven) |
| CORE-8 | progress reconciliation across rematching and multi-issue files | `[x]` covered by the unmatched and split tests, plus `mark_to_page_sets_position_and_completes_on_last_page` |
| CORE-9 | XML safety, filename parsing, match acceptance/rejection, offline fallback | `[~]` ComicInfo parsing and filename parsing are exercised by integration tests, but **the XML size cap / DTD refusal has no dedicated test**; providers do not exist |
| CORE-10 | reorder correctness, import/export round-trip, unresolved placeholders | `[~]` reorder covered; import/export does not exist |
| CORE-11 | spoiler-filtered search, pagination stability | `[x]` covered |
| CORE-13 | smart rule compilation (no injection), orphan cleanup | `[~]` injection covered; orphan cleanup N/A (FKs) |
| CORE-14 | every surface that can leak a spoiler | `[~]` tests cover `library_snapshot`, `issue`, `order_detail` and `search_filtered`; **`series_detail`, `collection_detail`, `reading_history` and `issue_variants` call `apply_spoilers` but have no leak test** |
| LATER | each feature needs its own accuracy and performance criteria before starting | `[~]` LATER-3 started: roster from `<Characters>` is tested (`import_records_characters_from_comic_info`, idempotency, spoiler filtering); portraits, alias UI, appearance types and cross-series merge are untested because they do not exist |

---

## 10. Risks that affect the roadmap

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Archive/parser vulnerabilities | Security, crash, data loss | Rust, limits, fuzzing, no path extraction |
| RAR licensing | Cannot ship CBR | Legal review before promising it; CBZ/folder first |
| WebView differences | Reader behaves differently on Linux | Per-platform testing; transcode unsupported formats |
| Large scans block UI | Unusable library | Background jobs, short transactions, incremental results |
| Thumbnail cache growth | Disk exhaustion | Content-addressed cache with size budget and eviction |
| Metadata providers change/disappear | Broken enrichment | Adapter isolation, cached data, no core dependency |
| Automated matching errors | Wrong progress, spoilers | Proposals + review + undo |
| Spoiler leaks | Core promise broken | Central filter, surface-by-surface tests |
| Reading-order data quality | Users lose trust | Curated orders, visible sources, no "one true chronology" claim |
| Migration failure | Data loss | Pre-migration backups, tested migrations from real data |
| Sync creep | Privacy and complexity | Sync is LATER; design change log, implement nothing yet |
| Scope growth in the reader | Endless polish, no release | Fixed budgets per phase; ship modes incrementally |

---

## 11. Decisions needed before implementation starts

These block Phase 0/1 and are repeated from `ARCHITECTURE.md` and `DATA_MODEL.md`:

Status: the first seven are effectively settled by the implementation; only D7 and the phase
order are still open.

1. `[x]` **D1** — Files and catalog issues are separate (`library_files` vs `issues`, joined by
   `issues.file_id`; v10 allows many issues per file).
2. `[x]` **D2** — SQLite + `rusqlite`.
3. `[x]` **D3** — `comic://` custom protocol for page delivery.
4. `[x]` **D5** — CBR/RAR included, via the pure-Rust `unrar-rs` crate.
5. `[x]` **D6** — Spoiler filtering is enforced in Rust (`repo::apply_spoilers`), not React.
6. `[ ]` **D7** — TypeScript binding generator. Still open; DTOs are handwritten.
7. `[~]` **Q1–Q8** — Answered implicitly by the schema as built; the document has not been
   updated to record the answers.
8. `[ ]` **Phase order** — Not revisited; the implemented order differs from §7 (orders and
   collections landed alongside search, spoilers are global not per-profile).

---

## 12. What "done" looks like for the first public CORE release

- [x] A user adds a folder, the library indexes, and every CBZ is readable without internet.
- [~] Single page, spread and continuous modes work with real zoom and keyboard control, but
  **per-series preferences only cover reading direction**, not mode/fit/background.
- [~] Bookshelf and series views stay responsive: progressive rendering keeps the DOM bounded,
  **but there is no windowed virtualiser and no measured test at 5,000 files**.
- [~] Progress resumes correctly, including after rematch and split. **Moving/replacing a file is
  not handled** (no relocate-root flow).
- [ ] Metadata is optional and reviewable, but **not provenance-tracked or provider-cached** —
  there are no providers. ComicInfo and filename parsing are the whole story.
- [~] Orders combine issues from any publishers and track progress. **No sections, no optional
  items, no import/export, and not per-profile.**
- [~] Search is fast and spoiler-safe on the surfaces it covers, but not every leak surface is
  tested.
- [~] Recovers from missing files and a failed migration (backup + test); **corrupt archives are
  a generic error**, and DB-busy has no designed state.
- [x] No telemetry, no cloud dependency, no required account.
