# Stdlib FS: `exists` の意味を path existence に揃えるか probe helper に縮退する

**Status**: open
**Created**: 2026-04-18
**Updated**: 2026-04-18
**ID**: 524
**Depends on**: none
**Track**: stdlib
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v{N}**: none
**Source**: false-done audit from `docs/stdlib/modernization/514-parser-host-quality-audit.md`

## Progress

- **2026-04-18**: Docs/contract slice — `std/host/fs.ark` `exists` doc comment + `std/manifest.toml` `doc` fields aligned with the read-probe contract; curated `modules/fs.md` overview in `generate-docs.py` calls out readable-file check vs path existence; regenerated `docs/stdlib/modules/fs.md`, `io.md`, and `reference.md`. (`std/fs/mod.ark` already matched this contract.) Runtime rename / semantics choice deferred.

## Summary

現行 `std::fs::exists` は doc comment 上も implementation 上も `__intrinsic_fs_read_file` 成功可否に近く、
一般的な “path exists” semantics ではなく “readable regular file probe” に寄っている。
この issue は product claim を 1 つに固定する: 真の existence semantics を実装するか、
現実に合わせて API / docs / examples を probe helper contract に縮退する。

## Why this must exist

- `docs/stdlib/modernization/514-parser-host-quality-audit.md` が naming-to-behavior gap を明示している
- broad FS issue `#049` は path/fs 全体で大きすぎ、`exists` mismatch が close gate になっていない
- user-visible API 名が reality より強い主張をしている

## Evidence source

- `docs/stdlib/modernization/514-parser-host-quality-audit.md`
- `std/fs/mod.ark`
- `docs/stdlib/modules/fs.md`

## Primary paths

- `std/fs/mod.ark`
- `tests/fixtures/stdlib_fs/`
- `tests/fixtures/manifest.txt`
- `docs/stdlib/modules/fs.md`
- `docs/stdlib/reference.md`

## Non-goals

- full filesystem capability rollout
- `read_dir` / metadata redesign
- `std::path` API expansion

## Acceptance

- [ ] repo chooses one semantics explicitly: true path-existence check or probe-style helper
- [ ] chosen semantics is implemented or renamed/documented consistently
- [ ] fixtures cover at least one file, one missing path, and one non-file or unreadable-path edge
- [ ] docs/reference/examples no longer imply stronger semantics than the implementation provides

## Required verification

- focused FS fixture run for the chosen contract
- `bash scripts/run/verify-harness.sh --fixtures`
- `python3 scripts/check/check-docs-consistency.py`

## Close gate

- the meaning of `exists` is repo-provable from code + fixtures + docs
- no user-visible page teaches “exists” semantics that the implementation does not satisfy
