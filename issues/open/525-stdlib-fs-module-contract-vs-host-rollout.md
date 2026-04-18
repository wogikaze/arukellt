# Stdlib FS: module-level docs を host rollout reality に合わせる

**Status**: open
**Created**: 2026-04-18
**Updated**: 2026-04-18
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

- [ ] module overview makes the current subset / bridge status explicit at import-time
- [ ] docs explain how `std::fs` relates to the broader `std::host::*` rollout without overclaiming durable surface area
- [ ] generated reference and curated module page do not contradict each other
- [ ] no user-visible page implies “complete filesystem facade” without repo proof

## Required verification

- `python3 scripts/gen/generate-docs.py`
- `python3 scripts/check/check-docs-consistency.py`

## Close gate

- docs-only claim is reduced to repo-provable wording
- generated and curated docs agree on the module contract
