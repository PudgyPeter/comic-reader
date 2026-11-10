# Panel — Comic Reader

A local-first desktop comic reader and library manager: Tauri 2 + React + TypeScript +
SQLite. Built against `design.md`, with architecture notes in `docs/`.

## Status

CORE (Stage 1 / first-release definition) is implemented end to end:

- Library import of CBZ/ZIP/CBR/PDF archives and image folders (indexed in place, files never
  modified), with per-file progress and cancellation
- ComicInfo.xml metadata, filename parsing, manual and bulk metadata editing
- Visual bookshelf with grid/list layouts, filters, sorting, progress and search
- Series organisation, custom collections, custom reading orders
- Reader with single page, double-page spreads, vertical scroll and horizontal strip
- Fit page/width/height/original, cursor-anchored zoom, panning, transitions
- Reading progress with a configurable completion threshold, resume, bookmarks with notes
- Spoiler protection driven by per-series progress or a chosen reading order
- Per-series and per-file reading direction (manga right-to-left), custom series covers
- Alternate covers and special editions linked to a base issue, missing-issue detection
- Reading history page, drag-and-drop ordering for collections and reading orders
- Configurable keyboard shortcuts, help panel, command palette
- Statistics dashboard and JSON metadata export

Characters are a first slice of the universe work: a series-scoped roster built from the
`<Characters>` tag in each issue's ComicInfo.xml, with descriptions that state their source
(the comics, the web, or you). It works offline; the web lookup is a per-click opt-in.

Stage 2/3 items (panel detection, full character/universe database, timeline, sync) are
explicitly out of scope for this release and remain follow-ups.

## Download

Windows builds are attached to [releases](https://github.com/PudgyPeter/comic-reader/releases).
The current installer is always:

```
https://github.com/PudgyPeter/comic-reader/releases/latest/download/Panel-setup.exe
```

and the portable build is `Panel.exe` on the same release. Builds are unsigned, so Windows
SmartScreen warns on first run.

To cut a release, tag a version and push it:

```bash
git tag v0.1.1
git push origin v0.1.1
```

``.github/workflows/release.yml` first runs typecheck, lint, Vitest, `cargo test` and clippy on
Ubuntu, then builds on `windows-latest` with the native MSVC toolchain and attaches the
installer plus the portable exe to a GitHub release with generated notes. The tag also sets
the version, so `v0.1.1` produces `Panel_0.1.1_x64-setup.exe`. Run the workflow manually
(`workflow_dispatch`) to verify the pipeline without publishing.

## Development

```bash
npm install
npm run dev        # frontend only (browser)
npm run tauri dev  # full desktop app
npm test           # Vitest
npm run typecheck
cd src-tauri && cargo test
```

### UI development fixture

The Linux sandbox used during development cannot render the webview reliably, so the UI can
run in a plain browser against a snapshot exported from the real Rust data layer:

```bash
npm run dev
# open http://localhost:1420/?mock#/
```

`src/mock/data.json` and `public/mock-pages/` are generated fixtures. Regenerate them with:

```bash
cd src-tauri
cargo run --example export       -- ~/.local/share/com.comicreader.app ../src/mock/data.json
cargo run --example export_pages -- ~/.local/share/com.comicreader.app ../public/mock-pages
```

If the local database is empty, seed it first:
`cargo run --example seed -- <db-dir> <library-dir>`, then
`cargo run --example organise -- <db-dir>`. Theme screenshots: append
`&theme=light` or `&theme=midnight` to the mock URL.

The mock bridge is only installed when the dev server sees `?mock` in the URL, and it is
excluded from production builds by `import.meta.env.DEV`.

## Data

- SQLite database + cache live in the platform app-data directory (identifier
  `com.comicreader.app`).
- A backup of the database is written before migrations run.
- Page and cover bytes are served through the `comic://` custom protocol, with covers cached
  on disk.
- Comic archives are opened read-only; metadata edits only touch the database.
