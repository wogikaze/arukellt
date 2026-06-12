---
Status: open
Created: 2026-03-31
Updated: 2026-06-12
ID: 422
Track: repo-hygiene
Depends on: 418
Orchestration class: implementation-ready
---

# Repo Hygiene: 大きな artifact と baseline の size budget / pruning ルールを作る

## Reopened by audit — 2026-06-12 (Slice F)

**Classification:** `must-reopen` / `acceptance-not-actually-met`

**Reopen reason:** Resolution cites `scripts/check/check-orphan-inventory.sh` (large-file
category) and `scripts/check/check-admission-gate.sh`. Both paths are absent. Policy
docs (`docs/retention-policy.md`, `docs/directory-ownership.md`) exist, but acceptance
item 2 (size measurement script/check) lacks repo proof.

**Violated acceptance:**
- サイズ計測スクリプトまたはチェックが追加される
- 少なくとも 1 つの CI / hook でサイズ情報が見える

**Evidence files:**
- Present: `docs/retention-policy.md`, `docs/directory-ownership.md`
- Missing: `scripts/check/check-orphan-inventory.sh`, `scripts/check/check-admission-gate.sh`
- Depends on reopened #418 for inventory script

**Follow-up split:** none (blocked by #418)

## Acceptance

- [x] 大型 artifact の予算または許容ルールが文書化される。
- [ ] サイズ計測スクリプトまたはチェックが追加される。
- [x] pruning の対象と残す理由の書式が決まる。
- [ ] 少なくとも 1 つの CI / hook でサイズ情報が見える。

## Resolution (prior — partial only)

- `docs/retention-policy.md` defines 1MB budget per file, binary assets prefer external hosting
- `.vscode-test/` excluded from large file scan (VS Code test downloads)
- `docs/directory-ownership.md` lists build artifacts as "never committed"

## Required verification

- Size measurement check exists and is wired into `manager.py verify quick` or CI
- Large-file report runnable without missing script paths

## Close gate

Size budget enforcement check passes in verify quick with repo-cited script path.
