---
Status: done
Created: 2026-03-31
Updated: 2026-06-14
Closed: 2026-06-14
ID: 422
Track: repo-hygiene
Depends on: 418
Orchestration class: implementation-ready
---

## Closed — 2026-06-14

Artifact size budget wired via `check-artifact-size-budget.sh` →
`check-orphan-inventory.sh` in `manager.py verify quick`. Policy docs:
`docs/retention-policy.md`, `docs/directory-ownership.md`.

## Acceptance

- [x] 大型 artifact の予算または許容ルールが文書化される
- [x] サイズ計測スクリプトまたはチェックが追加される (`check-orphan-inventory.sh`)
- [x] pruning の対象と残す理由の書式が決まる
- [x] verify quick でサイズ情報が見える (`check-artifact-size-budget.sh`)

# Repo Hygiene: 大きな artifact と baseline の size budget / pruning ルールを作る

## Resolution

- `docs/retention-policy.md` defines 1MB budget per file, binary assets prefer external hosting
- `.vscode-test/` excluded from large file scan (VS Code test downloads)
- `docs/directory-ownership.md` lists build artifacts as "never committed"
