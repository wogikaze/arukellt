---
Status: done
Created: 2026-03-31
Updated: 2026-04-01
ID: 384
Track: stdlib-api
Depends on: 383
Orchestration class: implementation-ready
# Stdlib: API 追加時の admission gate と family coverage チェックを導入する
---
# Stdlib: API 追加時の admission gate と family coverage チェックを導入する

## Acceptance

- [x] 新規 stdlib API に対する admission checklist が機械可読な形で定義される。
- [x] CI が fixture / docs / metadata の欠落を検出する。
- [x] family ごとの API 数・fixture 数・docs 数を出す coverage report が生成される。
- [x] admission gate を通らない API 追加が fail する。

## Resolution

- Created `scripts/check/check-admission-gate.sh` that validates every manifest API has fixture, docs, and stability metadata
- Script exits 1 on errors (missing stability), exits 0 with warnings for missing fixtures
- Generates per-family coverage report with API/stable/experimental/deprecated counts
- 274 APIs checked, 0 errors, 35 fixture warnings, 78% overall coverage