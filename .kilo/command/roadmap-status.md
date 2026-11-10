---
description: Audit docs/ROADMAP.md against the code and fix stale markers
---

Run the `roadmap-status` skill (`.kilo/skill/roadmap-status/SKILL.md`) now.

Scope hint from the user, if any: $ARGUMENTS

Steps:

1. If the hint is empty, derive the changed areas from `git status --short` and `git diff` —
   including uncommitted and staged work.
2. For each deliverable in §3–§6 that the change touches, verify the claim in the source and
   flip the marker with the correct semantics: `[x]` only with a named test or a recorded
   manual check in `docs/HANDOVER.md`; `[~]` with a bold note saying exactly what is missing;
   `[ ]` when nothing shipped.
3. Update every paired location the change invalidates: header tally, §2 state table, §7 phase
   suffixes and remaining-work paragraph, §8 cross-cutting rows, §9 testing rows, §11 decisions,
   §12 done-list.
4. Re-run the header counter from the skill and make the numbers match.

Report the bullets changed as a short list, with the evidence (test name or file:line) for each
`[x]`.
