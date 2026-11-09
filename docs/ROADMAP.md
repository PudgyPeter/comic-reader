# Comic Reader — Development Roadmap

> **Status:** CORE (Stage 1) partly implemented. Reading, library, series, collections,
> orders, search (FTS5 with filters), stats, spoiler filtering, smart collections and
> unmatched-file matching work offline; metadata providers and per-profile spoilers are not
> started. See
> [`HANDOVER.md`](./HANDOVER.md) for the live remaining-work list.
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

## 2. Current state

| Item | State |
| --- | --- |
| Tauri 2 shell | Implemented; 68 IPC commands, `comic://` asset protocol |
| React UI | Library, Series, Reader, Collections, Orders, Search, History, Stats, Settings |
| Rust modules | `db`, `library`, `archive`, `pdf`, `metadata`, `repo`, `protocol`, `commands`, `models` |
| Database | Schema v6 (FTS5 search index) with versioned migrations and pre-migration backups |
| Archive support | CBZ, ZIP, CBR, image PDF, image folders |
| Tests / lint | 34 Rust integration tests, 34 Vitest tests, clippy `-D warnings`, eslint. No CI pipeline yet. |
| Security config | Tightened CSP, scoped capability set |
| Docs | `ARCHITECTURE.md`, `DATA_MODEL.md`, `HANDOVER.md`, this file |

The task tables below were written before implementation; treat the state table above as
authoritative and use [`HANDOVER.md`](./HANDOVER.md) for the live remaining-work list.

---

## 3. CORE foundation (must precede feature work)

These are prerequisites, not features in the product sense.

| ID | Task | Why it is first |
| --- | --- | --- |
| F1 | Rust module skeleton, `AppError`, `spawn_blocking` conventions | Every command depends on one error and concurrency model |
| F2 | SQLite connection, pragmas, migration runner, backup-before-migrate | Data loss is unrecoverable without it |
| F3 | IPC DTO conventions and a generated-bindings spike (`specta` vs `ts-rs`) | Prevents silent contract drift from day one |
| F4 | `comic://` custom protocol with a stub handler | Page delivery is architectural; retrofitting it is painful |
| F5 | Frontend app shell: router, providers, error boundaries, feature folders | Giant components appear without this from the start |
| F6 | Query cache + typed `invoke` wrapper + error mapping | Sets the data-fetching pattern before any screen exists |
| F7 | Test harness: Vitest + Testing Library, Rust integration tests, synthetic CBZ fixtures | Untested archive and migration code is a liability |
| F8 | Lint/format/CI: ESLint, Prettier, `rustfmt`, `clippy -D warnings` | Cheap now, expensive later |
| F9 | Restrictive CSP and minimal capabilities; remove unused `opener` grant | The template's security posture is not shippable |
| F10 | Logging, diagnostics bundle, crash-safe startup recovery | Required to debug a desktop app you cannot inspect remotely |

**Sequencing note:** F1–F4 are backend-first; F5–F6 are frontend-first; F7–F10 are continuous.
Do not build a reader screen before F2/F4 exist, or you will rewrite it.

---

## 4. CORE features

### CORE-1 — Library import and file browsing

**Goal:** Point the app at folders, index files, browse them, and open any of them.

Deliverables:

- Add/remove library roots through a native folder picker; paths stay in Rust.
  **A root may be a single large archive file too (multi-GB omnibus CBZ/CBR/PDF).**
- Recursive scan with progress, cancellation, and resume.
- File records with format, size, page count, and status; unmatched files visible.
  **Unmatched files have a dedicated view + file reader (schema v9).**
- Manual rescan; missing/unreadable file states.
- Root-aware paths: move a root, keep the library.
- Background scanning that does not freeze the UI.
- CBZ/ZIP and image-folder support (folder support may trail CBZ by one increment).

Explicitly not in this increment: file watching, CBR/RAR, PDF.

**Risks:** scan performance on network drives; case-insensitive path collisions; huge archives.
**Acceptance:** a 5,000-file library indexes incrementally and remains browsable during the scan.

### CORE-2 — Reader engine: multiple reading modes

**Goal:** A reader that handles single page, double-page spread, and continuous vertical strip.

Deliverables:

- Reader state machine (`ARCHITECTURE.md` §12.2) and page manifest loading.
- **Single page** with fit-width / fit-height / fit-page / actual size.
- **Double-page spread** with correct pairing rules: cover alone, spreads aligned to odd/even,
  manual override per file, and RTL support for manga.
- **Continuous vertical** (webtoon) with virtualised page rendering.
- Spread overrides persisted in `file_preferences`.
- Preload of adjacent pages with cancellation.
- Clear loading/error states per page, not a blank screen.

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

- Zoom to point (cursor/pinch anchor), not zoom to center.
- Pan with pointer/trackpad, with clamping at page edges.
- Fit modes and a "reset zoom" command.
- Keyboard zoom shortcuts and a visible zoom indicator.
- Bounded decoded image size and cache; zoom never re-fetches an unbounded full-resolution page.
- Optional high-resolution re-request for zoomed-in views (a `maxWidth` bump), with a cache.

**Explicit non-goal:** deep-zoom tiling. This is a decision (D14). If real scans prove soft,
schedule tiling as a separate research task with a clear memory budget, not as a "zoom improvement".

**Acceptance:** zooming a 4,000×6,000 page to 400% shows real additional detail (because a
higher-resolution variant was requested), not a blurred CSS-scaled image.

### CORE-4 — Page transitions

**Goal:** Pleasant, fast page changes without interfering with reading.

Deliverables:

- None/slide/fade/cover transitions.
- Respect `prefers-reduced-motion`.
- No animation on rapid consecutive turns (animation must not lag behind input).
- Transition duration and behaviour configurable; disableable.
- No transition when preload has not completed; show a lightweight loading state instead.

**Criticism:** transitions are a common source of perceived slowness in comic readers. Treat
"instant" as the default and transitions as an option. Do not build a physics engine.

### CORE-5 — Reading customisation

**Goal:** Make the reader comfortable across devices and preferences.

Deliverables:

- Global, per-series, and per-file settings resolution (most specific wins).
- Reading mode, reading direction, fit mode, spread behaviour, background colour, brightness,
  gamma, and toolbar auto-hide.
- Page number/percentage display and a jump-to-page control.
- Full-screen and distraction-free mode.
- All controls keyboard-accessible; settings persist offline.

**Criticism:** preference layering is easy to implement badly. Use one resolver function with
tests, not scattered `series?.mode ?? global.mode ?? 'single'` expressions.

### CORE-6 — Visual bookshelf

**Goal:** A library that looks like a shelf and stays fast at thousands of items.

Deliverables:

- Cover grid with virtualisation and variable cover aspect ratios.
- Cover extraction and cached thumbnails (with placeholder for missing covers).
- Sort (title, added, release date, last read) and filters (format, read status, publisher).
- Continue-reading row and "recently added" row.
- List view as an alternative for large libraries.
- Empty, scanning, and error states.

**Risks:** thumbnail generation storms on first open; layout jank from mixed aspect ratios.
**Acceptance:** 5,000 covers scroll at interactive frame rates; thumbnails generate in the
background and never block the grid.

### CORE-7 — Series organisation

**Goal:** Group issues into series and browse them sensibly.

Deliverables:

- Series list and detail views with issues ordered by number, with natural sort.
- Manual issue reordering for specials/annuals that break numeric order.
- Series metadata editing with metadata lock. **Metadata lock implemented (schema v7);
  franchise grouping is not (publisher/universe only).**
- Publisher and franchise grouping.
- Variant handling as designed in `DATA_MODEL.md` (decision Q2).
- Unmatched files view with a "match to issue" flow. **Implemented (schema v9): `/unmatched`
  lists loose files (imported with "keep unmatched"), `/file/:id` reads them with file-scoped
  progress/bookmarks, and matching reuses the file row so progress survives. Manual matching
  only; provider-driven match proposals are still open.**

**Criticism:** automatic series grouping from filenames will be wrong often. Ship manual grouping
and correction before promising automation.

### CORE-8 — Reading tracker

**Goal:** Know what has been read and resume accurately.

Deliverables:

- Resume at last page (optionally with scroll offset in continuous mode).
- Mark as read/unread; mark up to a page; mark a whole series.
- Read status derived from file progress and reconciled to issue status.
- Continue-reading and completed lists.
- Progress survives rematching a file to an issue (with the explicit recompute rule from
  `DATA_MODEL.md` §6.4).
- Throttled, crash-safe progress writes.

**Criticism:** the file-vs-issue progress duality is the most likely source of user-visible bugs.
Write tests for the reconciliation before building UI on top.

### CORE-9 — Automatic metadata

**Goal:** Enrich the catalog from local embedded metadata first, remote providers second.

Deliverables:

- `ComicInfo.xml` parsing (safe XML, size limits).
- Filename parser with confidence scoring and a preview before applying.
- Provider adapters behind one trait, user-enabled, offline-tolerant, rate-limited.
- Match proposals with reasons; accept/reject; never silently overwrite user edits.
- Provenance display ("this synopsis came from X on date Y").
- Manual metadata editing with lock. **Lock implemented (schema v7); editing, automatic
  lock-on-save, lock-aware bulk edits and forced refresh exist. Providers are not built.**
- Cache and snapshot provider responses; all fetched data remains available offline.

**Criticisms:**

- Provider APIs change, impose rate limits, or disappear. Treat every provider as optional and
  keep the adapter interface small.
- Automated matching will merge the wrong issue. Keep it reviewable and reversible.
- Scaffolding an HTTP client and a provider abstraction is easy; the hard part is match quality
  and conflict resolution. Budget for that, not for the HTTP code.

### CORE-10 — Chronological reading order

**Goal:** Curated and (optionally) generated orders that mix publishers, series, and formats.

Deliverables:

- Create/edit/reorder an order of issues (and unmatched files).
- Sections/group labels ("Act I", "Before the war").
- Optional/skippable items.
- Per-profile order progress and next-unread item.
- Import/export of a versioned JSON order with UUIDs and unresolved placeholders.
- Explicit statement that orders are independent of series and publisher.
- No pretense of a single canonical chronology.

**Criticism:** "automatic chronological order" is not a solvable problem in general. In-universe
dates are inconsistent, flashbacks break naive sorting, and tie-ins are debatable. Ship curated
orders with optional generation from `story_start_date` as a draft. This is decision Q4.

### CORE-11 — Advanced search

**Goal:** Find things fast in a large local library.

Deliverables:

- FTS5 search over titles, creators, characters, arcs, events, publishers, and safe text.
  **Implemented** for titles, series, numbers, creators, publishers, descriptions and bookmark
  notes; character/arc/event text arrives with the universe schema.
- Filters: publisher, series, year range, read status, format, collection, character, arc.
- Sort options and keyset pagination.
- Search result snippets generated **after** spoiler filtering.
- Keyboard-first search with recent searches.

**Criticism:** spoiler-safe search is subtle. A character name in a result can spoil. Plan a
dedicated test suite with a profile whose frontier is mid-series.

### CORE-12 — Keyboard shortcuts

**Goal:** A reader that can be driven without a mouse.

Deliverables:

- Central command registry with scopes and user-remappable bindings.
- Default bindings for page turn, jump, zoom, mode, fullscreen, bookmark, and search.
- Conflict detection and a cheat sheet.
- Focus management in dialogs and the reader.
- Every important mouse action has a keyboard equivalent.

### CORE-13 — Custom collections

**Goal:** User-curated sets independent of reading orders.

Deliverables:

- Create/rename/delete collections with covers.
- Add series, issues, arcs, characters, events, orders, and unmatched files.
- Manual ordering and notes.
- A fixed, typed smart-collection rule set compiled to SQL (no arbitrary expressions).
  **Implemented (schema v8): 15 allowlisted fields × 10 operators, bound values, empty rules
  match nothing, smart membership flows through detail/counts/search filters.**
- Collection membership visible on entity detail pages.

### CORE-14 — Spoiler protection

**Goal:** Do not reveal future story information to the active profile.

Deliverables:

- `spoiler_read_marks` model from `DATA_MODEL.md` §13.4 (conservative v1).
- Per-profile spoiler mode; first-run choice of starting point (Q7).
- Hidden: synopses, character lists, arcs/events whose reveal is after the frontier, future-issue
  previews, and spoiler-sensitive search results.
- A visible indicator when content is hidden because of spoilers, with an explicit "reveal"
  action (which is recorded).
- The same filter used by detail views, list views, and search.
- Tests for every surface that can leak.

**Criticism:** this is the feature most likely to be declared done while still leaking. Define
the surfaces explicitly (issue detail, series detail, bookshelf hover, search, reading order
preview, character page, arc page, collections) and test each one. Do not rely on a `hidden` class.

---

## 5. LATER

These features depend on CORE data and should not begin until the corresponding CORE items are
stable. Several are research projects, not tickets.

### LATER-1 — Guided panel reading

Step through panels rather than pages: focus, zoom, and navigate panel-by-panel, with a page
overview.

**Why later:** it needs panel detection (LATER-2) and a whole new navigation model in the reader
state machine. It also interacts badly with double-page spreads and continuous mode.
**Decision needed:** is guided reading a separate mode or an overlay on the existing modes?

### LATER-2 — Panel detection

Automatic panel boundary detection for archives without panel metadata.

**Why later and why hard:** classical computer vision (edge/contour detection) is unreliable on
painted, full-bleed, or irregular layouts; ML models add a large dependency, platform variance,
and licensing questions. Expect manual correction to be necessary.
**Recommendation:** start with detection into an editable, per-page panel table; do not ship
detection as authoritative. Store results so failures are visible and fixable.

### LATER-3 — Character profiles

Character pages with descriptions, aliases, first appearance, portraits, and a list of
appearances.

**Depends on:** `characters`, `character_aliases`, `character_appearances` (schema now, UI later).
**Hard parts:** identity (same character across reboots/universes), data volume, spoiler-safe
appearance lists, and sourcing. Do not expect a complete character database from providers.

### LATER-4 — Relationship maps

A graph of character relationships with types and directions.

**Depends on:** `character_relationships`.
**Hard parts:** graph layout at scale, spoiler-filtered subgraphs (hide edges revealed later),
and representing changing relationships over time. A static graph over the whole run is
misleading; consider relationship states per era/arc.

### LATER-5 — Universe timeline

A visual timeline of events, arcs, and issues, with in-universe and publication views.

**Depends on:** `timelines`, `timeline_entries`, `events`, `arcs`.
**Hard parts:** precise dates are rare, concurrent storylines are common, and a timeline that
mixes publishers needs curatorship. Expect this to be a curated view, not an automatically
generated truth.

### LATER-6 — Story arc tracking

Arc progress across series and publishers, with "continue this arc" and arc-level spoiler state.

**Depends on:** `arcs`, `arc_issues`, reading progress.
**Hard parts:** arcs with optional tie-ins, reading order vs arc order, and marking an arc read
when its issues were read out of order.

### LATER-7 — Location database

Locations, appearances, and relationships (part of, located in, destroyed in).

**Depends on:** new `locations`, `location_appearances`, `location_relationships` tables (a purely
additive migration).
**Hard parts:** spoilers (a location's destruction is an event), naming collisions (multiple
"Gotham"s), and limited provider data. This is the least standardised domain; expect manual entry.

### LATER-8 — Character appearance tracker

A per-character timeline of appearances with filters and progress ("which appearances have I not
read?").

**Depends on:** `character_appearances`, spoiler read marks.
**Hard parts:** volume (thousands of rows per character), missing data, and the difference between
a cameo and a meaningful appearance. Needs pagination and an explicit data-quality indicator.

### LATER-9 — Multi-device synchronisation

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

### OPTIONAL-1 — Statistics dashboard

Reading time, pages read, issues completed, streaks, series breakdowns, and a yearly summary.

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
Phase 0   Foundations F1-F10
Phase 1   CORE-1 library import  ->  CORE-2 single-page reader (CBZ only)
Phase 2   CORE-6 bookshelf + CORE-7 series organisation
Phase 3   CORE-3 zoom + CORE-4 transitions + CORE-5 customisation
Phase 4   CORE-8 reading tracker + CORE-2 spread/continuous modes
Phase 5   CORE-9 metadata (ComicInfo first, providers second)
Phase 6   CORE-11 search + CORE-12 shortcuts
Phase 7   CORE-13 collections + CORE-10 reading orders
Phase 8   CORE-14 spoiler protection across all surfaces
Phase 9   Hardening: performance, accessibility, packaging, migrations, diagnostics
Phase 10  LATER features in the order that user demand justifies
```

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

| Area | Requirement |
| --- | --- |
| Offline | Every screen works with the network disabled; test this in CI or manually each phase |
| Performance | Budgets for cold start, library scroll, reader open, and memory; measure, do not guess |
| Accessibility | Keyboard-first, focus rings, screen reader labels, reduced motion, sufficient contrast |
| Error UX | Missing file, corrupt archive, DB busy, permission denied each have a designed state |
| Migrations | Every schema change has a migration, a backup step, and a test from the previous version |
| Security | New inputs (archives, XML, provider HTML) get limits and tests before release |
| Diagnostics | Logs and a support bundle that do not leak paths or keys |
| Packaging | Installers and code signing for each target; test on a clean machine |

---

## 9. Testing by stage

| Stage | Must be tested |
| --- | --- |
| Foundations | migration runner, backup/restore, protocol handler, error serialization |
| CORE-1 | scanner idempotency, cancellation, missing files, case collisions |
| CORE-2/3 | spread pairing, fit modes, zoom anchor math, memory bounds |
| CORE-5 | preference resolution precedence |
| CORE-6/7 | virtualisation, cover placeholders, natural sort, unmatched match keeps progress |
| CORE-8 | progress reconciliation across rematching and multi-issue files |
| CORE-9 | XML safety, filename parsing, match acceptance/rejection, offline fallback |
| CORE-10 | reorder correctness, import/export round-trip, unresolved placeholders |
| CORE-11 | spoiler-filtered search, pagination stability |
| CORE-13 | smart rule compilation (no injection), orphan cleanup |
| CORE-14 | every surface that can leak a spoiler |
| LATER | each feature needs its own accuracy and performance criteria before starting |

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

1. **D1** — Confirm files and catalog issues are separate (the foundation of the schema).
2. **D2** — Confirm SQLite + `rusqlite` rather than `sqlx`.
3. **D3** — Confirm the `comic://` custom protocol for page delivery.
4. **D5** — Decide whether CBR/RAR is required at launch (licensing).
5. **D6** — Confirm spoiler filtering is enforced in Rust/SQL, not React.
6. **D7** — Choose the TypeScript binding generator via a short spike.
7. **Q1–Q8** — The data-model questions in `DATA_MODEL.md` §20.
8. **Phase order** — Confirm whether reading orders move earlier than phase 7.

---

## 12. What "done" looks like for the first public CORE release

- A user adds a folder, the library indexes, and every CBZ is readable without internet.
- Single page, spread, and continuous modes work, with real zoom, per-series preferences, and
  keyboard control.
- The bookshelf and series views stay responsive at thousands of files.
- Reading progress resumes correctly, including after files are moved, rematched, or replaced.
- Metadata is optional, reviewable, provenance-tracked, and cached for offline use.
- Reading orders can combine any issues from any publishers and track progress independently.
- Search is fast and spoiler-safe.
- The app recovers cleanly from missing files, corrupt archives, and a failed migration.
- There is no telemetry, no cloud dependency, and no required account.
