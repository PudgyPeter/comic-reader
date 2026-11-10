---
name: roadmap-status
description: Keep docs/ROADMAP.md truthful. Use after any change that implements, partially implements, or exposes a roadmap deliverable, and whenever asked to update roadmap status. Covers marker semantics, the paired locations that must be updated together (header tally, §2 state table, §7 phases, §8 cross-cutting, §9 testing, §11 decisions, §12 done-list), and the evidence required before flipping a marker.
---

# Roadmap status bookkeeping

`docs/ROADMAP.md` is the single source of truth for what is built. It is only useful if it
matches the code, so **every turn that changes product behaviour ends with a roadmap pass**.
This is not optional cleanup; a stale marker is a bug in the roadmap.

## 1. When to run this

Run the pass at the end of any turn that:

- adds, removes, or changes a feature, command, route, setting, schema version, or test;
- exposes an already-built backend function through `api.ts` or a screen;
- closes a gap that a `[~]` bullet's bold note names;
- is explicitly asked to "mark off" / "update the roadmap".

Skip it only for changes with no deliverable behind them (typo fixes, docs-only edits to
`ARCHITECTURE.md`, dependency bumps that change nothing user-visible).

## 2. Marker semantics (do not invent new ones)

- `[x]` — done **and** covered by an automated test or a manual check recorded in
  `docs/HANDOVER.md`. If you cannot point at a test or a recorded check, it is not `[x]`.
- `[~]` — partially done. The bullet **must** end with a bold note naming exactly what is
  missing (`**No ... yet**`, `**Only ...**`). "Partial" with no note is not allowed.
- `[ ]` — not started, or started and reverted.

Where a bold note and a marker disagree, fix them so they agree; never leave the contradiction.

## 3. Evidence before flipping a marker

Verify in the source, not from memory or from the previous session's summary:

1. `grep`/`read` the actual implementation (Rust command + `repo` fn for backend claims; the
   page/component for UI claims).
2. Confirm the IPC/UI wiring end to end when the claim is "exposed": a `repo` function with no
   `commands.rs` entry, no `api.ts` method, or no caller is **not** done.
3. For a `[x]`, name the test: `grep -n "fn <test>" src-tauri/tests/core.rs` or a
   `src/**/*.test.ts` file. If none exists, either write the test in this turn or mark `[~]`
   with `**no test**` in the note.
4. Check for partial limits worth stating: hard caps (`MAX_*`), depth limits, max-width clamps,
   missing error/empty/loading states, unremappable keys, filters present in SQL but absent from
   the UI.

## 4. Every location that must move together

A single deliverable change usually touches more than one place. After editing a bullet, walk
this list and update any row it invalidates.

| Location | What to re-check |
| --- | --- |
| Header `> **Status:**` (line ~3) | Recount `[x]` / `[~]` / `[ ]` across §3–§6 and rewrite the "N of M deliverables" numbers plus the unstarted-feature sentence. |
| §2 Current state table | IPC command count, schema version, module list, archive formats, Rust/Vitest test counts, lint/CI state. |
| §3 Foundations F1–F10 | Frontend-backend conveniences, test harness, lint/format/CI, diagnostics. |
| §4 CORE-1…CORE-14 | The deliverable itself; split a bullet rather than half-marking one. |
| §5 LATER / §6 OPTIONAL | Only if the change starts one of those (`[ ]` → `[~]`), plus the inline `— \`[ ] not started\`` heading suffix. |
| §7 Delivery sequence | The per-phase `done (… missing)` suffix and the "Remaining CORE work, in dependency order" paragraph. |
| §8 Cross-cutting | Offline / performance / a11y / error UX / migrations / security / diagnostics / packaging rows. |
| §9 Testing by stage | The row for the touched area; move a claim from "not tested" to a named test. |
| §11 Decisions | Flip D7/Q1–Q8/phase-order only when the decision is actually recorded. |
| §12 "What done looks like" | The release checklist line the change satisfies. |

## 5. Counting the header tally

Markers also appear in §1 (legend), §5, §6, §8, §9, §11 and §12, so a naive `grep -c` is
wrong. Count only §3 through just before §5:

```bash
python3 - <<'EOF'
import re
from collections import Counter
lines=open('docs/ROADMAP.md').read().splitlines()
c=Counter(); inside=False
for l in lines:
    if re.match(r'## 3\.',l): inside=True
    if re.match(r'## 5\.',l): inside=False
    if inside and re.search(r'\[[x~ ]\]',l):
        for m in ('[x]','[~]','[ ]'):
            if m in l: c[m]+=1; break
print(dict(c), sum(c.values()))
EOF
```

Paste the output into the header sentence in the same shape:
`N of M deliverables done, P partial, U not started (foundations F1–F10 plus CORE-1…CORE-14)`.

## 6. House style for a corrected bullet

- Stay in the existing voice: a claim, then the evidence, then the caveat in bold.
- Name real symbols: `api.markIssueToPage`, `repo::apply_spoilers`, `library::MAX_SCAN_FILES`.
- Prefer a concrete consequence over an adjective: "a broken page renders as a blank box" beats
  "error handling is incomplete".
- Do not delete the original requirement text; rewrite the marker and append the reality.
- Keep lines inside the file's ~100-column wrapping.
- No emoji, no changelog entries, no "updated today" timestamps.

## 7. Finish

Re-run the counter, confirm the header matches, and re-read any bullet you flipped for
self-contradiction. Report the exact bullets changed in one short list — the roadmap diff is
the deliverable.
