---
Status: done
Created: 2026-04-18
Updated: "2026-04-18 (docs slice landed)"
Track: main
Orchestration class: implementation-ready
Depends on: none
---
# Stdlib FS: module-level docs を host rollout reality に合わせる
**Closed**: 2026-04-18
**ID**: 525
**Depends on**: none
**Track**: stdlib, docs
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v1 exit**: no
**Source**: false-done audit from `docs/stdlib/modernization/514-parser-host-quality-audit.md`

## Summary

`std::fs` namespace は module 名だけ見ると一般 filesystem facade に見える一方、current implementation は
temporary bridge / partial host-backed subset の側面が強い。 limitation が import-time docs で十分に見えないため、
module-level docs と generated reference を `std::host::*` rollout reality に合わせて明示する。

## Why this must exist

- `docs/stdlib/modernization/514-parser-host-quality-audit.md` が module header mismatch を follow-up 化している
- implementation issue と docs realignment を分けないと、docs-only drift が再発する
- current queue に `std::fs` module contract wording だけを close gate にした issue がない

## Evidence source

- `docs/stdlib/modernization/514-parser-host-quality-audit.md`
- `docs/stdlib/modules/fs.md`
- `docs/current-state.md`
- `std/fs/mod.ark`

## Primary paths

- `docs/stdlib/modules/fs.md`
- `docs/current-state.md`
- `docs/stdlib/reference.md`
- `std/fs/mod.ark`

## Non-goals

- filesystem runtime implementation changes
- `exists` semantic changes
- capability target-gating changes

## Acceptance

- [x] module overview makes the current subset / bridge status explicit at import-time
- [x] docs explain how `std::fs` relates to the broader `std::host::*` rollout without overclaiming durable surface area
- [x] generated reference and curated module page do not contradict each other
- [x] no user-visible page implies “complete filesystem facade” without repo proof

## Progress

- **2026-04-18:** `std/fs/mod.ark` module docs; curated `modules/fs.md` overview + `build_target_constraints` fix (no more “No host capability required” alongside `--dir` **Availability**); regenerated `docs/stdlib/modules/fs.md` and `docs/stdlib/modules/io.md`; cross-link from `docs/current-state.md` Known Limitations.
- **Verified:** `bash scripts/run/verify-harness.sh --quick`, `python3 scripts/gen/generate-docs.py`, `python3 scripts/check/check-docs-consistency.py`.

## Required verification

- `python3 scripts/gen/generate-docs.py`
- `python3 scripts/check/check-docs-consistency.py`

## Close gate

- docs-only claim is reduced to repo-provable wording
- generated and curated docs agree on the module contract

---

## Close note — 2026-04-18

Closed as complete. Module-level docs updated to reflect host rollout reality without overclaiming durable surface area.

**Close evidence:**
- Docs slice landed 2026-04-18
- `std/fs/mod.ark` module docs updated
- Curated `modules/fs.md` overview added with `build_target_constraints` fix
- Regenerated `docs/stdlib/modules/fs.md` and `docs/stdlib/modules/io.md`
- Cross-link from `docs/current-state.md` Known Limitations
- All 4 acceptance criteria checked
- Verification: `bash scripts/run/verify-harness.sh --quick` → PASS (2026-04-18)
- Verification: `python3 scripts/gen/generate-docs.py` → PASS (2026-04-18)
- Verification: `python3 scripts/check/check-docs-consistency.py` → PASS (2026-04-18)

**Acceptance mapping:**
- ✓ Module overview makes subset/bridge status explicit at import-time
- ✓ Docs explain `std::fs` relation to `std::host::*` rollout without overclaiming
- ✓ Generated reference and curated module page don't contradict
- ✓ No user-visible page implies "complete filesystem facade" without repo proof

**Implementation notes:**
- This is a docs-only slice; no filesystem runtime implementation changes
- Module contract wording reduced to repo-provable claims
- Fixed "No host capability required" alongside `--dir` Availability contradiction