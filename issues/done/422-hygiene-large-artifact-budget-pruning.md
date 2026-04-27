---
Status: done
Created: 2026-03-31
Updated: 2026-04-01
ID: 422
Track: repo-hygiene
Depends on: —
Orchestration class: implementation-ready
---

# Repo Hygiene: 大きな artifact と baseline の size budget / pruning ルールを作る
- `scripts/check/check-orphan-inventory.sh` reports files > 500KB (category 1: large files)
# Repo Hygiene: 大きな artifact と baseline の size budget / pruning ルールを作る

## Acceptance

- [x] 大型 artifact の予算または許容ルールが文書化される。
- [x] サイズ計測スクリプトまたはチェックが追加される。
- [x] pruning の対象と残す理由の書式が決まる。
- [x] 少なくとも 1 つの CI / hook でサイズ情報が見える。

## Resolution

- `docs/retention-policy.md` defines 1MB budget per file, binary assets prefer external hosting
- `scripts/check/check-orphan-inventory.sh` reports files > 500KB (category 1: large files)
- `.vscode-test/` excluded from large file scan (VS Code test downloads)
- `docs/directory-ownership.md` lists build artifacts as "never committed"
- `scripts/check/check-admission-gate.sh` provides family coverage info