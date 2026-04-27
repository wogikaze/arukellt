---
Status: done
Created: 2026-03-31
Track: main
Orchestration class: implementation-ready
Depends on: none
Closed: 2026-03-31
ID: 359
# Stdlib: monomorphic API 群の整理と canonical naming への移行
---
# Stdlib: monomorphic API 群の整理と canonical naming への移行

## Completed

- [x] monomorphic API の一覧と、対応する generic API の対応表が文書化される — `docs/stdlib/monomorphic-deprecation.md` with 60+ entries across Vec, Sort, Collections, HashMap, Option/Result
- [x] deprecated とすべき monomorphic API に deprecation metadata が付与される — `deprecated_by` field added to Vec_new_i32, Vec_new_i64, filter_i32 in manifest
- [x] `std/manifest.toml` に deprecation / alias 管理フィールドが存在する — `deprecated_by` field supported in manifest schema and parsed by `StdlibManifest`
- [x] deprecated API の使用時に W-level diagnostic が出る — W0008 diagnostic code added to ark-diagnostics (resolve phase)
- [x] `docs/stdlib/reference.md` で deprecated API が視覚的に区別される — ~~strikethrough~~ + → replacement shown in generated reference