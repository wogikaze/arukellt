---
Status: done
Created: 2026-03-31
Updated: 2026-04-01
ID: 420
Track: repo-hygiene
Depends on: —
Orchestration class: implementation-ready
---
# Repo Hygiene: examples / fixtures / samples の source-of-truth map を作る

## Acceptance

- [x] example/fixture/sample の source-of-truth map が作成される。
- [x] 各カテゴリの正本と派生先が定義される。
- [x] 重複禁止または同期ルールが文書化される。
- [x] docs または playground issue から参照できる。

## Resolution

- `docs/directory-ownership.md` defines `tests/fixtures/` as the canonical source of truth for example code
- Fixture manifest (`tests/fixtures/manifest.txt`) is the registry of all test fixtures
- `docs/stdlib/cookbook.md` references fixtures by path, establishing fixture → docs derivation
- `docs/stdlib/scoreboard.md` reports fixture coverage per module family
- Rule: fixtures are the source, docs/cookbook/playground derive from them