# Agent notes

## Commands

- `npm test` — Vitest unit tests (`src/**/*.test.ts`).
- `npm run typecheck` — `tsc --noEmit`.
- `npm run build` — type-check plus Vite production build.
- `cd src-tauri && cargo test` — Rust integration tests (`tests/core.rs`).
- `cd src-tauri && cargo check` — fast compile check.
- `npm run lint` — ESLint over `src/`.
- `npm run clippy` — Rust lints (`-D warnings`).

## Layout

- `src-tauri/src`: `db` (schema/migrations), `library` (scan/import), `repo` (queries and
  commands backing), `archive` (zip/page/thumbnail), `metadata` (ComicInfo + filename),
  `protocol` (`comic://`), `commands` (Tauri IPC surface), `models` (DTOs).
- `src/pages`: one file per route; `src/components`: shell, UI primitives, editors.
- `src/lib`: typed API client, shared types, utilities, spread pairing, drag reorder helper.
- `src/mock` + `public/mock-pages`: generated UI fixture, ignored by git.

## Conventions

- IPC DTO field names are camelCase on the wire; Rust structs use `serde(rename_all)`.
- All archive/database work runs in Rust via `spawn_blocking`; the frontend never reads files.
- Comic files are never written to. Metadata lives only in SQLite.
- Keep the spoiler filter in `repo::apply_spoilers`; every query that surfaces issues must go
  through it.
- `library::DISPLAY_FIELDS` and `map_issue_offset` are positional; update both together.
- Schema changes bump `SCHEMA_VERSION` in `db.rs` and add a new `MIGRATION_N` block; never edit
  a shipped migration (existing user databases have already run it).

## Working style (streaming reliability)

Long turns get truncated by the client ("response ended unexpectedly"). Keep every turn small:

- **Never emit more than ~150 lines in a single file-write tool call.** Create the file with a
  short skeleton, then append in chunks with `edit`/`bash cat >>`. Same for large JSX/Rust edits.
- **One concern per turn.** Finish a step, run its check, report, then continue. Do not chain
  five unrelated refactors and a 900-line build inside one message.
- **Never run a build/test inside a long reasoning turn.** Run `cargo build`, `cargo test`,
  `npm run build`, and `npm install` as their own tool call with an explicit `timeout`, then
  read the tail. Use the `monitor` background action for anything over ~2 minutes.
- **Keep tool output small.** Always pipe build/test output through `tail`/`grep`; never dump
  full compiler logs or whole generated files into the transcript.
- **Prefer targeted reads.** Read the 40 lines around a symbol instead of whole files; use
  `grep` to locate first.
- **Checkpoint before risky work.** After a green `tsc`/`cargo check`, say so in one line so the
  session can resume from a known-good state if it drops.
