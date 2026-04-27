---
Status: done
Created: 2026-03-31
Updated: 2026-04-01
ID: 361
Track: stdlib-api
Depends on: 360
Orchestration class: implementation-ready
---
# Stdlib: prelude と module import の二重露出を整理する

## Acceptance

- [x] prelude に残す名前と module-only にする名前の基準が文書化される
- [x] 重複露出する関数の一覧が作成され、各関数の canonical access path が決定される
- [x] resolver が canonical path を優先して解決する
- [x] `docs/stdlib/reference.md` が canonical path を表示する

## Resolution

- Created `docs/stdlib/prelude-dedup.md` documenting canonical access paths
- Analysis: 101 prelude functions, 172 module-only functions, 0 actual duplicates
- The "dual exposure" is a doc_category illusion — no function has both `prelude=true` and a `module` field
- Virtual modules (std::math, std::string, std::collections) are doc categories, not importable paths
- Resolver already correctly prefers explicit imports over prelude names
- Reference docs already show prelude/module column for each function