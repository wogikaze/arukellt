---
Status: open
Created: 2026-03-28
Updated: 2026-04-22
ID: 099
Track: selfhost-frontend
Depends on: —
Orchestration class: design-ready
---

# Selfhost compiler: incremental parse design slice
Blocks v4 exit: no
Status note: Selfhost frontend design lane. Do not group with #125/#126 trusted-base compiler default-path work or #285/#508/#529 legacy-removal work.
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
Selfhost compiler work for incremental parsing: define how unchanged source modules are detected, how parse results are reused, and how dependent compilation units are invalidated in the Ark compiler sources under `src/compiler/*.ark`.
4. The verification line points at the current repo manager check: `python3 scripts/manager.py verify`.
STOP_IF: the requested parser-cache skeleton lives in `src/compiler/*.ark`, which is selfhost frontend work rather than compiler-core work. Implementing even a minimal cache API here would require crossing into the selfhost parser/driver/module graph, and the issue text does not define a safe, bounded data-structure-only slice. Leave product code unchanged and hand this to the selfhost lane instead.
---
# Selfhost compiler: incremental parse design slice


---

## Reopened by audit - 2026-04-03


**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/099-compile-incremental-parse.md` — incorrect directory for an open issue.


## Summary

Selfhost compiler work for incremental parsing: define how unchanged source modules are detected, how parse results are reused, and how dependent compilation units are invalidated in the Ark compiler sources under `src/compiler/*.ark`.

## Dispatchable slice

This issue is ready to hand to a selfhost compiler implementation agent as a narrow design slice.

## Acceptance criteria

1. The issue scope names `src/compiler/*.ark` as the implementation path for incremental parse work.
2. The acceptance text describes parse reuse and invalidation in selfhost terms, not Rust-crate cache plumbing.
3. The issue does not require implementing compiler caching, mtime-based reuse, or watch-mode plumbing.
4. The verification line points at the current repo manager check: `python3 scripts/manager.py verify`.
5. Any companion doc wording, if added later, stays within `docs/compiler/` or `docs/current-state.md`.

## Notes

- This slice is design-ready only.
- Do not treat old `crates/*` wording as the active implementation target for this issue.

## Blocker note - 2026-04-22

STOP_IF: the requested parser-cache skeleton lives in `src/compiler/*.ark`, which is selfhost frontend work rather than compiler-core work. Implementing even a minimal cache API here would require crossing into the selfhost parser/driver/module graph, and the issue text does not define a safe, bounded data-structure-only slice. Leave product code unchanged and hand this to the selfhost lane instead.

## Responsibility split — 2026-04-22

\#099 is **selfhost frontend design** work. It should be dispatched to the
selfhost frontend lane when a bounded design/implementation slice exists. It is
not blocked by, and should not block, #125/#126 trusted-base compiler default
path correction or #285/#508/#529 legacy removal / selfhost transition planning.